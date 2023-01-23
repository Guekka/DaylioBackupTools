//! This module interprets the parsed PDF data into a Daylio struct.

use crate::parse_pdf::{DayEntry, ParsedPdf, StatLine};
use chrono::NaiveDateTime;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone, Default)]
struct ProcessedDayEntry {
    date: NaiveDateTime,
    mood: i64,
    tags: Vec<i64>,
    note: String,
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default)]
struct Mood {
    id: i64,
    name: String,
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default)]
struct Tag {
    id: i64,
    name: String,
}

#[derive(Debug, PartialEq, Clone, Default)]
struct ProcessedPdf {
    day_entries: Vec<ProcessedDayEntry>,
    original_moods: Vec<StatLine>,
    moods: Vec<Mood>,
    tags: Vec<Tag>,
}

fn parse_date(entry: &DayEntry) -> Result<NaiveDateTime> {
    // Date looks like August 2, 2022
    // Time looks like Monday 8 45 PM

    // Ignore the day of the week
    let time_idx = entry
        .day_hour
        .find(' ')
        .ok_or_else(|| eyre!("Invalid time"))?;
    let time = &entry.day_hour[time_idx + 1..];

    // August 2, 2022 8:45 PM
    let time_str = format!("{} {}", entry.date, time);

    NaiveDateTime::parse_from_str(&time_str, "%B %e, %Y %l %M %p")
        .map_err(|e| eyre!("Failed to parse date: {}", e))
}

/// Extracts tags from the note, and returns the note with the tags removed.
fn extract_tags(entry: &DayEntry, stats: &Vec<StatLine>) -> (String, Vec<String>) {
    let mut entry_tags = Vec::new();

    let mut last_tag_line = None;
    for (i, line) in entry.note.iter().enumerate() {
        for tag in stats {
            // tag comparison is case sensitive
            if line.contains(&tag.name) {
                entry_tags.push(tag.name.clone());
                last_tag_line = Some(i);
            }
        }
        if last_tag_line != Some(i) {
            break;
        }
    }

    let note = if let Some(last_tag_line) = last_tag_line {
        let mut note = entry.note.clone();
        note.drain(..=last_tag_line);
        note
    } else {
        entry.note.clone()
    };

    (note.join("\n"), entry_tags)
}

fn list_tags_and_moods(parsed: &ParsedPdf) -> (Vec<Tag>, Vec<Mood>) {
    let mut moods: HashSet<Mood> = HashSet::with_capacity(parsed.stats.len());
    let mut tags: HashSet<Tag> = HashSet::with_capacity(parsed.stats.len());

    for entry in &parsed.day_entries {
        let (_, entry_tags) = extract_tags(entry, &parsed.stats);
        moods.insert(Mood {
            id: moods.len() as i64,
            name: entry.mood.clone(),
        });

        for tag in entry_tags {
            tags.insert(Tag {
                id: tags.len() as i64,
                name: tag,
            });
        }
    }

    (tags.into_iter().collect(), moods.into_iter().collect())
}

impl From<ParsedPdf> for ProcessedPdf {
    fn from(parsed: ParsedPdf) -> Self {
        let (tags, moods) = list_tags_and_moods(&parsed);

        let day_entries = parsed
            .day_entries
            .into_iter()
            .map(|entry| {
                let date = parse_date(&entry).unwrap();
                let (note, entry_tags) = extract_tags(&entry, &parsed.stats);

                let entry_mood = moods.iter().find(|x| x.name == entry.mood).unwrap().id;
                let entry_tags = entry_tags
                    .iter()
                    .map(|x| tags.iter().find(|y| y.name == *x).unwrap().id)
                    .collect();

                ProcessedDayEntry {
                    date,
                    mood: entry_mood,
                    tags: entry_tags,
                    note,
                }
            })
            .collect();

        // preserve original moods for their order
        let mut original_moods = parsed.stats;
        original_moods.truncate(moods.len());

        ProcessedPdf {
            day_entries,
            original_moods,
            moods,
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_parse_date() {
        let entry = DayEntry {
            date: "August 2, 2022".to_owned(),
            day_hour: "Monday 8 45 PM".to_owned(),
            mood: String::new(),
            note: vec![],
        };
        let date = parse_date(&entry).unwrap();
        assert_eq!(date.month(), 8);
        assert_eq!(date.day(), 2);
        assert_eq!(date.year(), 2022);
        assert_eq!(date.hour(), 20);
        assert_eq!(date.minute(), 45);
        assert_eq!(date.second(), 0);
    }

    impl StatLine {
        fn with_name(name: &str) -> Self {
            StatLine {
                name: name.to_owned(),
                count: 0,
            }
        }
    }

    #[test]
    fn test_extract_tags() {
        let entry = DayEntry {
            date: String::new(),
            day_hour: String::new(),
            mood: String::new(),
            note: vec![
                "some tag   another tag    yet another tag ".to_owned(),
                "A tag, on another line".to_owned(),
                "A tag that does not matches case".to_owned(),
                "not a tag".to_owned(),
                "still not a tag".to_owned(),
            ],
        };

        let stats = vec![
            StatLine::with_name("some tag"),
            StatLine::with_name("another tag"),
            StatLine::with_name("yet another tag"),
            StatLine::with_name("A tag, on another line"),
            StatLine::with_name("A tag that does not matches CASE"),
        ];
        let (note, tags) = extract_tags(&entry, &stats);

        let expected_note = vec![
            "A tag that does not matches case".to_owned(),
            "not a tag".to_owned(),
            "still not a tag".to_owned(),
        ]
        .join("\n");
        let expected_tags = vec![
            "some tag".to_owned(),
            "another tag".to_owned(),
            "yet another tag".to_owned(),
            "A tag, on another line".to_owned(),
        ];

        assert_eq!(note, expected_note);
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_processed_pdf_from_parsed_pdf() {
        let parsed = ParsedPdf {
            day_entries: vec![
                DayEntry {
                    date: "August 2, 2022".to_owned(),
                    day_hour: "Monday 8 45 PM".to_owned(),
                    mood: "Happy".to_owned(),
                    note: vec!["This is a note".to_owned()],
                },
                DayEntry {
                    date: "August 3, 2022".to_owned(),
                    day_hour: "Tuesday 9 00 AM".to_owned(),
                    mood: "Sad".to_owned(),
                    note: vec![
                        "some tag   another tag    yet another tag ".to_owned(),
                        "Note title".to_owned(),
                        "Note body".to_owned(),
                    ],
                },
            ],
            stats: vec![
                StatLine::with_name("Happy"),
                StatLine::with_name("Sad"),
                StatLine::with_name("some tag"),
                StatLine::with_name("another tag"),
                StatLine::with_name("yet another tag"),
                StatLine::with_name("Tag that won't be matched"),
            ],
        };

        let expected = ProcessedPdf {
            day_entries: vec![
                ProcessedDayEntry {
                    date: parse_date(&parsed.day_entries[0]).unwrap(),
                    mood: 0,
                    tags: vec![],
                    note: "This is a note".to_owned(),
                },
                ProcessedDayEntry {
                    date: parse_date(&parsed.day_entries[1]).unwrap(),
                    mood: 1,
                    tags: vec![0, 1, 2],
                    note: "Note title\nNote body".to_owned(),
                },
            ],
            original_moods: vec![StatLine::with_name("Happy"), StatLine::with_name("Sad")],
            moods: vec![
                Mood {
                    id: 0,
                    name: "Happy".to_owned(),
                },
                Mood {
                    id: 1,
                    name: "Sad".to_owned(),
                },
            ],
            tags: vec![
                Tag {
                    id: 0,
                    name: "some tag".to_owned(),
                },
                Tag {
                    id: 1,
                    name: "another tag".to_owned(),
                },
                Tag {
                    id: 2,
                    name: "yet another tag".to_owned(),
                },
            ],
        };

        let processed = ProcessedPdf::from(parsed);

        pretty_assertions_sorted::assert_eq!(processed, expected);
    }
}
