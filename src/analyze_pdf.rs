//! This module interprets the parsed PDF data into a Daylio struct.

use chrono::{Datelike, NaiveDateTime, NaiveTime, Timelike};
use color_eyre::{Result, eyre};

use crate::parse_pdf::{DayEntry, ParsedPdf, StatLine};
use crate::{Daylio, NUMBER_OF_PREDEFINED_MOODS, daylio, merge};

#[derive(Debug, PartialEq, Clone, Default)]
struct ProcessedDayEntry {
    date: NaiveDateTime,
    mood: i64,
    tags: Vec<i64>,
    note: String,
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default, Ord, PartialOrd)]
struct Mood {
    id: i64,
    name: String,
    group: i64,
    predefined: bool,
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default, Ord, PartialOrd)]
struct Tag {
    id: i64,
    name: String,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub(crate) struct ProcessedPdf {
    day_entries: Vec<ProcessedDayEntry>,
    moods: Vec<Mood>,
    tags: Vec<Tag>,
}

fn convert_24_hour_to_12_hour(time_str: &str) -> Result<String> {
    let date_parts = time_str.split_whitespace().collect::<Vec<_>>();

    if date_parts.len() < 2 {
        eyre::bail!("Invalid date format: {}", time_str);
    }

    let mut hour = date_parts[0].to_owned();
    let minute = date_parts[1];

    let am_pm = if date_parts.len() == 3 {
        date_parts[2]
    } else {
        // 24h clock
        let hour_int = hour.parse::<u8>().unwrap();
        if hour_int > 12 {
            hour = (hour_int - 12).to_string();
            "pm"
        } else {
            "am"
        }
    };

    // sanitize hour
    if hour == "00" {
        hour = "12".to_owned();
    }

    Ok(format!("{hour} {minute} {am_pm}"))
}

fn parse_date(entry: &DayEntry) -> Result<NaiveDateTime> {
    // skip the day of the week
    let mut time_str = entry
        .day_hour
        .split_whitespace()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ");

    // sometimes hour is hour:minute, sometimes it's hour minute
    time_str = time_str.replace(':', " ");
    time_str = convert_24_hour_to_12_hour(&time_str)?;

    let time = NaiveTime::parse_from_str(&time_str, "%l %M %p")?;
    Ok(NaiveDateTime::new(entry.date, time))
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

fn predefined_mood_idx(custom_name: &str) -> Option<i64> {
    match custom_name.to_lowercase().as_ref() {
        "super" | "rad" => Some(1),
        "bien" | "good" => Some(2),
        "mouais" | "meh" => Some(3),
        "mauvais" | "bad" => Some(4),
        "horrible" | "awful" => Some(5),
        _ => None,
    }
}

fn update_mood_category(moods: &mut [Mood]) {
    let mut prev_id = None;
    for mood in moods {
        if let Some(idx) = predefined_mood_idx(&mood.name) {
            mood.id = idx;
            mood.predefined = true;
            prev_id = Some(idx);
        }
        mood.group = prev_id.unwrap_or(0);
    }
}

fn list_tags_and_moods(parsed: &ParsedPdf) -> (Vec<Tag>, Vec<Mood>) {
    let mut moods: Vec<Mood> = Vec::new();
    let mut tags: Vec<Tag> = Vec::new();

    for entry in &parsed.day_entries {
        let (_, entry_tags) = extract_tags(entry, &parsed.stats);
        if !moods.iter().any(|m| m.name == entry.mood) {
            moods.push(Mood {
                id: moods.len() as i64 + NUMBER_OF_PREDEFINED_MOODS,
                name: entry.mood.clone(),
                group: 0,
                predefined: false,
            });
        }

        for tag in entry_tags {
            if !tags.iter().any(|t| t.name == tag) {
                tags.push(Tag {
                    id: tags.len() as i64,
                    name: tag,
                });
            }
        }
    }

    // sort moods according to the order they appear in the PDF
    let mut moods: Vec<Mood> = moods.into_iter().collect();
    moods.sort_by_key(|mood| parsed.stats.iter().position(|stat| stat.name == mood.name));
    update_mood_category(&mut moods);

    (tags.into_iter().collect(), moods)
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

        ProcessedPdf {
            day_entries,
            moods,
            tags,
        }
    }
}

impl From<Mood> for daylio::CustomMood {
    fn from(mood: Mood) -> Self {
        daylio::CustomMood {
            id: mood.id,
            predefined_name_id: if mood.predefined { mood.id } else { -1 },
            custom_name: if mood.predefined {
                String::new()
            } else {
                mood.name
            },
            mood_group_id: mood.group,
            icon_id: 1,
            ..Default::default()
        }
    }
}

impl From<Tag> for daylio::Tag {
    fn from(tag: Tag) -> Self {
        daylio::Tag {
            id: tag.id,
            name: tag.name,
            icon: 1,
            ..Default::default()
        }
    }
}

impl From<ProcessedDayEntry> for daylio::DayEntry {
    fn from(entry: ProcessedDayEntry) -> Self {
        daylio::DayEntry {
            minute: i64::from(entry.date.minute()),
            hour: i64::from(entry.date.hour()),
            day: i64::from(entry.date.day()),
            month: i64::from(entry.date.month()) - 1, // month is 0-indexed in Daylio
            year: i64::from(entry.date.year()),
            datetime: entry.date.and_utc().timestamp_millis(),
            mood: entry.mood,
            note: entry.note,
            tags: entry.tags,
            ..Default::default()
        }
    }
}

impl From<ProcessedPdf> for Daylio {
    fn from(pdf: ProcessedPdf) -> Self {
        merge(
            Daylio::default(),
            Daylio {
                custom_moods: pdf.moods.into_iter().map(From::from).collect(),
                tags: pdf.tags.into_iter().map(From::from).collect(),
                day_entries: pdf.day_entries.into_iter().map(From::from).collect(),
                ..Default::default()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, Timelike};

    use super::*;

    #[test]
    fn test_parse_date() {
        let entry = DayEntry {
            date: NaiveDate::from_ymd_opt(2022, 8, 2).unwrap(),
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
            date: NaiveDate::from_ymd_opt(2022, 9, 2).unwrap(),
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

        let expected_note = [
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
                    date: NaiveDate::from_ymd_opt(2022, 9, 2).unwrap(),
                    day_hour: "Monday 8 45 PM".to_owned(),
                    mood: "rad".to_owned(),
                    note: vec!["This is a note".to_owned()],
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2022, 9, 3).unwrap(),
                    day_hour: "Tuesday 8 45 AM".to_owned(),
                    mood: "rad".to_owned(),
                    note: vec!["This is a note²".to_owned()],
                },
                DayEntry {
                    date: NaiveDate::from_ymd_opt(2022, 9, 3).unwrap(),
                    day_hour: "Tuesday 9 00 AM".to_owned(),
                    mood: "good".to_owned(),
                    note: vec![
                        "some tag   another tag    yet another tag ".to_owned(),
                        "Note title".to_owned(),
                        "Note body".to_owned(),
                    ],
                },
            ],
            stats: vec![
                StatLine::with_name("rad"),
                StatLine::with_name("good"),
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
                    mood: 1,
                    tags: vec![],
                    note: "This is a note".to_owned(),
                },
                ProcessedDayEntry {
                    date: parse_date(&parsed.day_entries[1]).unwrap(),
                    mood: 1,
                    tags: vec![],
                    note: "This is a note²".to_owned(),
                },
                ProcessedDayEntry {
                    date: parse_date(&parsed.day_entries[2]).unwrap(),
                    mood: 2,
                    tags: vec![0, 1, 2],
                    note: "Note title\nNote body".to_owned(),
                },
            ],
            moods: vec![
                Mood {
                    id: 1,
                    name: "rad".to_owned(),
                    group: 1,
                    predefined: true,
                },
                Mood {
                    id: 2,
                    name: "good".to_owned(),
                    group: 2,
                    predefined: true,
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

        assert_eq!(processed, expected);
    }
}
