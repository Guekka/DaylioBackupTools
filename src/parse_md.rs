//! Parses a Markdown diary. This is not exactly related to Daylio, and is tailored to my
//! personal needs.

use crate::models::{DayEntry, Diary};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use color_eyre::eyre::Result;
use regex::Regex;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

static DATE_TIME_REGEX: LazyLock<Regex, fn() -> Regex> = // yyyy-mm-dd
    LazyLock::new(|| {
        Regex::new(
            r"(?x)
            # delimiter
            \[
            # optional yyyy-mm-dd, ended by space
            (?:
              (?P<y>\d{4}) # the year
              -
              (?P<m>\d{2}) # the month
              -
              (?P<d>\d{2}) # the day
              \x20 # space
            )?
            # mandatory hh:mm
            (?P<hh>\d{2})
            [:h]
            (?P<mm>\d{2})
            # end delimiter
            ]
            ",
        )
        .unwrap()
    });

pub(crate) fn parse_md(input: &str) -> Diary {
    // entries are separated by a date in the format[YYYY-MM-DD HH:MM], with one of day and hour optional

    let day_entries = split_entries(input);

    let day_entries = forward_fill_dates(day_entries)
        .into_iter()
        .map(|(date, note)| make_entry(date, note))
        .collect::<Vec<_>>();

    Diary {
        day_entries,
        moods: vec![],
        tags: vec![],
    }
}

fn forward_fill_dates(
    entries: Vec<(Option<NaiveDate>, NaiveTime, String)>,
) -> Vec<(NaiveDateTime, String)> {
    // for this case, we assume all dates are complete
    if entries.len() < 2 {
        return entries
            .into_iter()
            .map(|(date, time, note)| (date.unwrap().and_time(time), note))
            .collect();
    }

    let mut previous_day: Option<NaiveDate> = None;
    let mut dated_entries = Vec::new();

    for (day, hm, note) in entries {
        let ymd = day.unwrap_or_else(|| {
            if let Some(previous_day) = previous_day {
                previous_day
            } else {
                panic!("No date found and no previous date to use");
            }
        });

        let date = NaiveDateTime::new(ymd, hm);
        dated_entries.push((date, note));
        previous_day = Some(ymd);
    }

    dated_entries
}

fn make_entry(date: NaiveDateTime, note: String) -> DayEntry {
    DayEntry {
        date,
        mood: None,
        tags: HashSet::new(),
        note,
    }
}

fn split_entries(input: &str) -> Vec<(Option<NaiveDate>, NaiveTime, String)> {
    let boundaries_dates = input
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            let captures = DATE_TIME_REGEX.captures(line)?;

            // optional date
            let date = if captures.name("y").is_some() {
                NaiveDate::from_ymd_opt(
                    captures["y"].parse::<i32>().unwrap(),
                    captures["m"].parse::<u32>().unwrap(),
                    captures["d"].parse::<u32>().unwrap(),
                )
            } else {
                None
            };

            // mandatory time
            let time = NaiveTime::from_hms_opt(
                captures["hh"].parse::<u32>().unwrap(),
                captures["mm"].parse::<u32>().unwrap(),
                0,
            )
            .unwrap();

            Some((line_num, date, time))
        })
        .collect::<Vec<_>>();

    if boundaries_dates.is_empty() {
        return Vec::new();
    }

    let extract_entry = |start, end, date, time| {
        let body = &input
            .lines()
            .skip(start + 1)
            .take(end - start - 1)
            .collect::<Vec<_>>()
            .join("\n");

        (date, time, body.trim().to_string())
    };

    let mut entries: Vec<(Option<NaiveDate>, NaiveTime, String)> = boundaries_dates
        .windows(2)
        .map(|window| {
            let [(start, start_date, start_time), (end, _, _)] = window else {
                panic!("Window should have exactly two elements");
            };
            extract_entry(*start, *end, *start_date, *start_time)
        })
        .collect();

    let last_entry = boundaries_dates.last().unwrap();
    let last_entry = extract_entry(
        last_entry.0,
        input.lines().count(),
        last_entry.1,
        last_entry.2,
    );

    entries.push(last_entry);
    entries
}

pub(crate) fn load_md(path: &Path) -> Result<Diary> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    Ok(parse_md(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;

    #[test]
    fn test_parse_md() -> Result<()> {
        // Given
        const INPUT: &str = r#"[2023-10-01 12:00]
Full date
[12:00]
No date, deduced from previous

[2025-10-01 10:00]
Make sure

we keep

whitespace
"#;

        // When
        let parsed = parse_md(INPUT);

        // Then
        let expected = Diary {
            moods: vec![],
            day_entries: vec![
                DayEntry {
                    date: chrono::NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    mood: None,
                    tags: HashSet::new(),
                    note: "Full date".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    mood: None,
                    tags: HashSet::new(),
                    note: "No date, deduced from previous".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2025, 10, 1)
                        .unwrap()
                        .and_hms_opt(10, 0, 0)
                        .unwrap(),
                    mood: None,
                    tags: HashSet::new(),
                    note: "Make sure\n\nwe keep\n\nwhitespace".to_string(),
                },
            ],
            tags: Vec::new(),
        };

        assert_eq!(parsed, expected);
        Ok(())
    }
}
