//! Parses a Markdown diary. This is not exactly related to Daylio, and is tailored to my
//! personal needs.

use crate::Daylio;
use crate::analyze_pdf::{Mood, ProcessedDayEntry, ProcessedPdf};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use color_eyre::Report;
use color_eyre::eyre::Result;
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

static BASE_DATE_REGEX: LazyLock<Regex, fn() -> Regex> =
    LazyLock::new(|| Regex::new(r"^\[(.*?)]$").unwrap());

static TIME_REGEX: LazyLock<Regex, fn() -> Regex> =
    LazyLock::new(|| Regex::new(r"(\d{2})[:h](\d{2})").unwrap());

static DATE_REGEX: LazyLock<Regex, fn() -> Regex> = // yyyy-mm-dd
    LazyLock::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap());

pub(crate) fn parse_md(input: &str) -> Result<ProcessedPdf> {
    // entries are separated by a date in the format[YYYY-MM-DD HH:MM], with one of day and hour optional

    let day_entries = input
        .lines()
        .fold(Vec::<(String, String)>::new(), split_entries)
        .into_iter()
        .collect::<Vec<_>>();

    let day_entries = forward_fill_dates(day_entries)
        .into_iter()
        .map(|(date, note)| make_entry(date, note))
        .collect::<Result<Vec<_>>>()?;

    Ok(ProcessedPdf {
        day_entries,
        moods: vec![Mood {
            id: 10,
            name: String::from("Inconnu"),
            group: 0,
            predefined: false,
        }],
        tags: Vec::new(),
    })
}

fn forward_fill_dates(entries: Vec<(String, String)>) -> Vec<(NaiveDateTime, String)> {
    if entries.len() < 2 {
        return entries
            .into_iter()
            .map(|(date, note)| {
                let date = dateparser::parse_with_timezone(&date, &chrono::offset::Utc)
                    .expect("Invalid date");
                (date.naive_local(), note)
            })
            .collect();
    }

    let mut previous_day: Option<NaiveDate> = None;
    let mut dated_entries = Vec::new();

    for (date, note) in entries {
        // parse date. day is optional
        let (day, time) = date.split_once(' ').unwrap_or(("", date.as_str()));

        let ymd = if day.is_empty() {
            None
        } else {
            Some(
                DATE_REGEX
                    .captures(day)
                    .and_then(|caps| {
                        let year = caps[1].parse::<i32>().ok()?;
                        let month = caps[2].parse::<u32>().ok()?;
                        let day = caps[3].parse::<u32>().ok()?;
                        NaiveDate::from_ymd_opt(year, month, day)
                    })
                    .unwrap_or_else(|| {
                        panic!("Invalid date format: {}. Expected format: YYYY-MM-DD", day)
                    }),
            )
        };

        let ymd = ymd.unwrap_or_else(|| {
            if let Some(previous_day) = previous_day {
                previous_day
            } else {
                panic!("No date found and no previous date to use");
            }
        });

        let hm = TIME_REGEX
            .captures(time)
            .and_then(|caps| {
                let hour = caps[1].parse::<u32>().ok()?;
                let minute = caps[2].parse::<u32>().ok()?;
                NaiveTime::from_hms_opt(hour, minute, 0)
            })
            .unwrap_or_else(|| panic!("Invalid time format: {}. Expected format: HH:MM", time));

        let date = NaiveDateTime::new(ymd, hm);
        dated_entries.push((date, note));

        previous_day = Some(ymd);
    }

    dated_entries
}

fn make_entry(date: NaiveDateTime, note: String) -> Result<ProcessedDayEntry, Report> {
    Ok(ProcessedDayEntry {
        date,
        mood: 10,
        tags: vec![],
        note,
    })
}

fn split_entries(mut entries: Vec<(String, String)>, line: &str) -> Vec<(String, String)> {
    if let Some(caps) = BASE_DATE_REGEX.captures(line) {
        entries.push((caps[1].to_string(), String::new()));
    } else {
        if let Some((_date, body)) = entries.last_mut() {
            if body.is_empty() {
                *body = line.to_string();
            } else {
                *body += "\n";
                *body += line;
            }
        }
    }
    // trim final whitespaces
    entries
        .into_iter()
        .map(|(date, body)| (date, body.trim().to_owned()))
        .collect()
}

pub(crate) fn load_md(path: &Path) -> Result<Daylio> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    parse_md(&data).map(|processed| processed.try_into())?
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
"#;

        // When
        let parsed = parse_md(INPUT)?;

        // Then
        let expected: ProcessedPdf = ProcessedPdf {
            day_entries: vec![
                ProcessedDayEntry {
                    date: chrono::NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    mood: 10,
                    tags: vec![],
                    note: "Full date".to_string(),
                },
                ProcessedDayEntry {
                    date: NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    mood: 10,
                    tags: vec![],
                    note: "No date, deduced from previous".to_string(),
                },
            ],
            moods: vec![Mood {
                id: 10,
                name: String::from("Inconnu"),
                group: 0,
                predefined: false,
            }],
            tags: Vec::new(),
        };

        assert_eq!(parsed, expected);
        Ok(())
    }
}
