//! Parses a Markdown diary. This is not exactly related to Daylio, and is tailored to my
//! personal needs.

use crate::models::{DayEntry, Diary, MdMetadata, Mood, Tag};
use crate::{MoodDetail, TagDetail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use color_eyre::eyre::Result;
use nom::IResult;
use nom::Parser;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{char, line_ending};
use nom::combinator::opt;
use nom::sequence::{delimited, terminated};
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
    let (input, header) = opt(parse_yaml_header).parse(input).unwrap();

    // entries are separated by a date in the format[YYYY-MM-DD HH:MM], with one of day and hour optional
    let day_entries = split_entries(input);

    let day_entries = forward_fill_dates(day_entries)
        .into_iter()
        .map(|(date, note)| make_entry(date, note))
        .collect::<Vec<_>>();

    let mut moods = header.as_ref().map(|h| h.moods.clone()).unwrap_or_default();
    let mut tags = header.as_ref().map(|h| h.tags.clone()).unwrap_or_default();
    // add unknown moods and tags from entries to the header lists
    for entry in &day_entries {
        for mood in &entry.moods {
            if !moods.iter().any(|m| m.name == mood.name) {
                moods.push(MoodDetail {
                    name: mood.name.clone(),
                    icon_id: None,
                    wellbeing_value: None,
                    category: None,
                });
            }
        }
        for tag in &entry.tags {
            if !tags.iter().any(|t| t.name == tag.name) {
                tags.push(TagDetail {
                    name: tag.name.clone(),
                    icon_id: None,
                });
            }
        }
    }

    Diary {
        day_entries,
        moods: header.as_ref().map(|h| h.moods.clone()).unwrap_or_default(),
        tags: header.as_ref().map(|h| h.tags.clone()).unwrap_or_default(),
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

fn read_mood_line(input: &str) -> IResult<&str, &str> {
    terminated(
        delimited(char('{'), take_until("}"), char('}')),
        line_ending,
    )
    .parse(input)
}

fn read_tag_line(input: &str) -> IResult<&str, &str> {
    terminated(
        delimited(tag("#{"), take_until("}"), char('}')),
        line_ending,
    )
    .parse(input)
}

fn make_entry(date: NaiveDateTime, note: String) -> DayEntry {
    // First line may contain mood in the form: {Mood / Mood2}
    // and tags in the form: #{Tag1,Tag2}
    let (remaining, (moods, tags)) = (opt(read_mood_line), opt(read_tag_line))
        .parse(&note)
        .unwrap();

    let moods = moods
        .map(|mood_line| {
            mood_line
                .split('/')
                .map(|mood_name| mood_name.trim())
                .map(Mood::new)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let tags = tags
        .map(|tag_line| {
            tag_line
                .split(',')
                .map(|tag_name| tag_name.trim())
                .map(Tag::new)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    DayEntry {
        date,
        moods,
        tags,
        note: remaining.trim().to_owned(),
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

fn parse_yaml_header(input: &str) -> IResult<&str, MdMetadata> {
    tag("---\n")
        .and(take_until("\n---"))
        .and(tag("\n---"))
        .map(|((_, yaml_content), _)| {
            let metadata: MdMetadata =
                serde_yaml::from_str(yaml_content).expect("Failed to parse YAML header");
            metadata
        })
        .parse(input)
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
    use crate::{MoodDetail, TagDetail};
    use color_eyre::Result;
    use similar_asserts::assert_eq;

    #[test]
    fn test_read_mood_line() {
        let input = "{Happy / Excited}\nThis is a test note.";
        let (remaining, mood_line) = read_mood_line(input).unwrap();
        assert_eq!(mood_line, "Happy / Excited");
        assert_eq!(remaining, "This is a test note.");
    }

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

[2025-10-02 11:00]
{Happy / Excited}
#{Work,Personal}
This is a mood and tags test.

[2025-10-03 09:30]
{Sad}
No tags here.

Just a sad entry.

[2025-10-04 14:15]
#{Urgent}
And this time, only a tag.
"#;

        // When
        let parsed = parse_md(INPUT);

        // Then
        let expected = Diary {
            moods: vec![],
            day_entries: vec![
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    moods: HashSet::new(),
                    tags: HashSet::new(),
                    note: "Full date".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2023, 10, 1)
                        .unwrap()
                        .and_hms_opt(12, 0, 0)
                        .unwrap(),
                    moods: HashSet::new(),
                    tags: HashSet::new(),
                    note: "No date, deduced from previous".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2025, 10, 1)
                        .unwrap()
                        .and_hms_opt(10, 0, 0)
                        .unwrap(),
                    moods: HashSet::new(),
                    tags: HashSet::new(),
                    note: "Make sure\n\nwe keep\n\nwhitespace".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2025, 10, 2)
                        .unwrap()
                        .and_hms_opt(11, 0, 0)
                        .unwrap(),
                    moods: vec![Mood::new("Happy"), Mood::new("Excited")]
                        .into_iter()
                        .collect(),
                    tags: vec![Tag::new("Work"), Tag::new("Personal")]
                        .into_iter()
                        .collect(),
                    note: "This is a mood and tags test.".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2025, 10, 3)
                        .unwrap()
                        .and_hms_opt(9, 30, 0)
                        .unwrap(),
                    moods: vec![Mood::new("Sad")].into_iter().collect(),
                    tags: HashSet::new(),
                    note: "No tags here.\n\nJust a sad entry.".to_string(),
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2025, 10, 4)
                        .unwrap()
                        .and_hms_opt(14, 15, 0)
                        .unwrap(),
                    moods: HashSet::new(),
                    tags: vec![Tag::new("Urgent")].into_iter().collect(),
                    note: "And this time, only a tag.".to_string(),
                },
            ],
            tags: Vec::new(),
        };

        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn test_parse_md_with_header() -> Result<()> {
        // Given
        const INPUT: &str = r#"---
moods:
    - name: Happy
      wellbeing_value: 5
    - name: Sad
      wellbeing_value: 1
tags:
    - name: Work
    - name: Personal
---
[2023-10-01 12:00]
{Happy / Sad}
#{Work,Personal}
Full date entry.
"#;

        // When
        let parsed = parse_md(INPUT);

        // Then
        let expected = Diary {
            moods: vec![
                MoodDetail {
                    name: "Happy".to_owned(),
                    icon_id: None,
                    wellbeing_value: Some(5),
                    category: None,
                },
                MoodDetail {
                    name: "Sad".to_owned(),
                    icon_id: None,
                    wellbeing_value: Some(1),
                    category: None,
                },
            ],
            tags: vec![
                TagDetail {
                    name: "Work".to_owned(),
                    icon_id: None,
                },
                TagDetail {
                    name: "Personal".to_owned(),
                    icon_id: None,
                },
            ],
            day_entries: vec![DayEntry {
                date: NaiveDate::from_ymd_opt(2023, 10, 1)
                    .unwrap()
                    .and_hms_opt(12, 0, 0)
                    .unwrap(),
                moods: vec![Mood::new("Happy"), Mood::new("Sad")]
                    .into_iter()
                    .collect(),
                tags: vec![Tag::new("Work"), Tag::new("Personal")]
                    .into_iter()
                    .collect(),
                note: "Full date entry.".to_string(),
            }],
        };

        assert_eq!(parsed, expected);
        Ok(())
    }
}
