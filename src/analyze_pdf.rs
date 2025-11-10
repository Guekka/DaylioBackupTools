//! This module interprets the parsed PDF data into a Diary struct.

use crate::daylio_predefined_mood_idx;
pub(crate) use crate::models::{DayEntry, Mood};
use crate::models::{Diary, MoodDetail, Tag, TagDetail};
use crate::parse_pdf::{ParsedDayEntry, ParsedPdf};
use chrono::{NaiveDateTime, NaiveTime};
use color_eyre::{Result, eyre};
use std::collections::HashSet;

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
        let hour_int = hour.parse::<u8>()?;
        if hour_int > 12 {
            hour = (hour_int - 12).to_string();
            "pm"
        } else {
            "am"
        }
    };

    // sanitize hour
    if hour == "00" {
        "12".clone_into(&mut hour);
    }

    Ok(format!("{hour} {minute} {am_pm}"))
}

fn parse_date(entry: &ParsedDayEntry) -> Result<NaiveDateTime> {
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
/// Most of the work should already be done by the parser,
/// but in some cases it might not be able to detect the tags.
/// The logic here is to detect tags by checking if the line contains only known tags.
fn extract_tags(entry: &ParsedDayEntry, all_tags: &[TagDetail]) -> (String, Vec<String>) {
    let mut tags_by_decreasing_length: Vec<TagDetail> = all_tags.to_owned();
    // sort the tags by length, so we can remove the longest ones first in case of overlap
    tags_by_decreasing_length.sort_unstable_by(|a, b| b.name.len().cmp(&a.name.len()));

    let mut entry_tags = Vec::new();

    let mut last_tag_line = None;
    for (i, line) in entry.note.iter().enumerate() {
        let mut line = line.to_owned();
        let mut line_tags = Vec::new();
        // detect tags in line
        for tag in &tags_by_decreasing_length {
            // tag comparison is case-sensitive
            if line.contains(&tag.name) {
                line_tags.push(tag.name.clone());
                // removing the tag is not very efficient, but probably not a big deal
                line.clone_from(&line.replace(&tag.name, ""));
            }
        }
        // make sure we only have tags in this line
        if line.trim().is_empty() {
            // this line only contained tags
            entry_tags.extend(line_tags);
            last_tag_line = Some(i);
        } else {
            // we have reached the end of the tags
            break;
        }
    }

    // remove the tags from the note
    let mut note_lines = entry.note.clone();
    if let Some(last_tag_line) = last_tag_line {
        note_lines.drain(..=last_tag_line);
    }

    // add tags detected by the parser, making sure they're valid. Try to guess the tags
    // if one is invalid
    let mut parsed_tags = entry.tags.clone();
    while !parsed_tags.is_empty() {
        let parsed_tag = parsed_tags.pop().unwrap();

        if all_tags
            .iter()
            .any(|x| x.name.to_lowercase() == parsed_tag.to_lowercase())
        {
            entry_tags.push(parsed_tag);
        } else {
            for tag in &tags_by_decreasing_length {
                // maybe two tags were mistakenly concatenated
                if parsed_tag.to_lowercase().contains(&tag.name.to_lowercase()) {
                    entry_tags.push(tag.name.clone());
                    // remove the tag from the note
                    let remaining_parsed_tag = parsed_tag.replace(&tag.name, "").trim().to_owned();

                    println!(
                        "Guessed tag {} from {}. Adding remaining to pending: {}",
                        tag.name, parsed_tag, remaining_parsed_tag
                    );

                    if !remaining_parsed_tag.is_empty() {
                        parsed_tags.push(remaining_parsed_tag);
                    }

                    break;
                }
            }
        }
    }

    // dedup (TODO: this should not be necessary)
    entry_tags.sort();
    entry_tags.dedup();

    (note_lines.join("\n"), entry_tags)
}

fn update_mood_category(moods: &mut [MoodDetail]) {
    let mut prev_id = None;
    for mood in moods {
        if let Some(idx) = daylio_predefined_mood_idx(&mood.name) {
            prev_id = Some(idx);
        }
        mood.wellbeing_value = u64::try_from(prev_id.unwrap_or(1)).ok();
    }
}

fn split_tags_and_moods(parsed: &ParsedPdf) -> (Vec<TagDetail>, Vec<MoodDetail>) {
    let max_mood_index: usize = parsed
        .day_entries
        .iter()
        .map(|entry| entry.mood.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|mood| {
            parsed
                .stats
                .iter()
                .position(|stat| stat.name.to_lowercase() == mood.to_lowercase())
                .expect("Entry mood not found in stats")
        })
        .max()
        .expect("Failed to find max mood index");

    // moods and tags are stored in the same vector, so we need to separate them
    // moods appear first, then tags
    let mut owned_stats = parsed.stats.clone();
    let (moods, tags) = owned_stats.split_at_mut(max_mood_index + 1);

    let mut moods = moods
        .iter()
        .map(|stat| MoodDetail {
            name: stat.name.clone(),
            icon_id: None,
            wellbeing_value: None,
            category: None,
        })
        .collect::<Vec<_>>();

    update_mood_category(&mut moods);

    let tags: Vec<TagDetail> = tags
        .iter_mut()
        .map(|stat| TagDetail {
            name: stat.name.clone(),
            icon_id: None,
        })
        .collect();

    (tags, moods)
}

/// Simplifies the note by removing unnecessary newlines and spaces. In particular:
/// - Remove some weird ligatures
/// - Removes newlines before end of sentence punctuation
/// - Removes newlines after dashes
/// - Replaces char\nchar with char char if both chars are lowercase
/// - Removes double or more spaces
/// - Removes spaces before dots
fn simplify_note_heuristically(mut text: String) -> String {
    const END_OF_SENTENCE_PUNCTUATION: [char; 3] = ['.', '!', '?'];

    // Normalize some unicode (ligature) characters
    text = text
        .replace("ﬃ", "ffi")
        .replace("ﬂ", "fl")
        .replace("ﬁ", "fi")
        .replace("ﬀ", "ff");

    let mut current_text = text.as_str();

    let mut simplified = String::new();
    loop {
        let newline_pos = current_text.find('\n');
        if newline_pos.is_none() {
            simplified.push_str(current_text);
            break;
        }
        let newline_pos = newline_pos.unwrap();

        let text_before_newline = &current_text[..newline_pos];
        let text_starting_at_newline = &current_text[newline_pos..];

        // Replaces char\nchar with char char if both chars are lowercase
        let previous_char = text_before_newline.chars().last().unwrap_or('\0');
        let next_char = text_starting_at_newline.chars().nth(1).unwrap_or('\0');

        if previous_char.is_lowercase() && next_char.is_lowercase() {
            simplified.push_str(&current_text[..newline_pos]);
            simplified.push(' ');
            current_text = &current_text[newline_pos + 1..];
            continue;
        }

        let previous_meaningful_char = text_before_newline
            .chars()
            .rev()
            .find(|c| !c.is_whitespace())
            .unwrap_or('\0');

        let next_meaningful_char = text_starting_at_newline
            .chars()
            .find(|c| !c.is_whitespace())
            .unwrap_or('\0');

        match (previous_meaningful_char, next_meaningful_char) {
            // Remove whitespace after dashes
            ('-', _) => {
                simplified.push_str(text_before_newline);
                let next_meaningful_char_pos =
                    text_starting_at_newline.find(next_meaningful_char).unwrap();
                current_text = &text_starting_at_newline[next_meaningful_char_pos..];
            }
            // Remove newlines before end of sentence punctuation
            (_, c) if END_OF_SENTENCE_PUNCTUATION.contains(&c) => {
                simplified.push_str(text_before_newline);
                current_text = &text_starting_at_newline[1..];
            }
            // Everything else, just keep as is
            _ => {
                simplified.push_str(text_before_newline);
                simplified.push('\n');
                current_text = &text_starting_at_newline[1..];
            }
        }
    }

    // Remove double or more spaces. Using a loop is not a good idea performance-wise, but
    // it should not matter much here since we are not dealing with huge strings and the loop
    // will not iterate more than a few times.
    loop {
        let new_simplified = simplified.replace("  ", " ");
        if new_simplified == simplified {
            break;
        }
        simplified = new_simplified;
    }

    // Remove spaces before dots
    simplified = simplified.replace(" .", ".");

    simplified
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

impl TryFrom<ParsedPdf> for Diary {
    type Error = eyre::Error;
    fn try_from(parsed: ParsedPdf) -> std::result::Result<Self, Self::Error> {
        let (tags, moods) = split_tags_and_moods(&parsed);

        let processed_entries = parsed
            .day_entries
            .iter()
            .map(|entry| extract_tags(entry, &tags))
            .collect::<Vec<_>>();

        let day_entries: Vec<DayEntry> = parsed
            .day_entries
            .into_iter()
            .enumerate()
            .map(|(entry_idx, entry)| {
                let date = parse_date(&entry).unwrap();
                let (note, entry_tags) = &processed_entries[entry_idx];
                let note = simplify_note_heuristically(note.clone());

                let entry_mood = moods
                    .iter()
                    .find(|x| x.name.to_lowercase() == entry.mood.to_lowercase())
                    .expect("Entry mood not found in moods")
                    .clone();

                let entry_tags: HashSet<Tag> = entry_tags.iter().map(|t| Tag::new(t)).collect();

                DayEntry {
                    date,
                    moods: HashSet::from([Mood::new(&entry_mood.name)]),
                    tags: entry_tags,
                    note,
                }
            })
            .collect();

        Ok(Diary {
            day_entries,
            moods,
            tags,
        }
        .sorted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_pdf::StatLine;
    use chrono::{Datelike, NaiveDate, Timelike};
    use similar_asserts::assert_eq;

    #[test]
    fn test_simplify_note_heuristically() {
        let text = r"Newline before punctuation
. Newline after over-
ride some spaces    and newline mid
sentence. Unicode ligature ﬃ

Preserve the empty line, but not the final one

";
        println!("original: {}", text);

        let simplified = simplify_note_heuristically(text.to_owned());
        assert_eq!(
            simplified,
            "Newline before punctuation. Newline after over-ride some spaces and newline mid sentence. Unicode ligature ffi\n\nPreserve the empty line, but not the final one"
        );

        assert_eq!(
            simplify_note_heuristically(
                "Elle ne peut plus parler et faire des gestes simples, mais ce\nn'est pas."
                    .to_owned()
            ),
            "Elle ne peut plus parler et faire des gestes simples, mais ce n'est pas."
        );
    }

    #[test]
    fn test_parse_date() {
        let entry = ParsedDayEntry {
            date: NaiveDate::from_ymd_opt(2022, 8, 2).unwrap(),
            day_hour: "Monday 8 45 PM".to_owned(),
            mood: String::new(),
            note: vec![],
            tags: vec![],
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
        let entry = ParsedDayEntry {
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
            tags: vec![],
        };

        let tags = vec![
            TagDetail {
                name: "some tag".to_owned(),
                icon_id: None,
            },
            TagDetail {
                name: "another tag".to_owned(),
                icon_id: None,
            },
            TagDetail {
                name: "yet another tag".to_owned(),
                icon_id: None,
            },
            TagDetail {
                name: "A tag, on another line".to_owned(),
                icon_id: None,
            },
        ];
        let (note, tags) = extract_tags(&entry, &tags);

        let expected_note = [
            "A tag that does not matches case".to_owned(),
            "not a tag".to_owned(),
            "still not a tag".to_owned(),
        ]
        .join("\n");
        let expected_tags = vec![
            "A tag, on another line".to_owned(),
            "another tag".to_owned(),
            "some tag".to_owned(),
            "yet another tag".to_owned(),
        ];

        assert_eq!(note, expected_note);
        assert_eq!(tags, expected_tags);
    }

    #[test]
    fn test_processed_pdf_from_parsed_pdf() {
        let parsed = ParsedPdf {
            day_entries: vec![
                ParsedDayEntry {
                    date: NaiveDate::from_ymd_opt(2022, 9, 2).unwrap(),
                    day_hour: "Monday 8 45 PM".to_owned(),
                    mood: "rad".to_owned(),
                    note: vec!["This is a note".to_owned()],
                    tags: vec![],
                },
                ParsedDayEntry {
                    date: NaiveDate::from_ymd_opt(2022, 9, 3).unwrap(),
                    day_hour: "Tuesday 8 45 AM".to_owned(),
                    mood: "rad".to_owned(),
                    note: vec!["This is a note²".to_owned()],
                    tags: vec![],
                },
                ParsedDayEntry {
                    date: NaiveDate::from_ymd_opt(2022, 9, 3).unwrap(),
                    day_hour: "Tuesday 9 00 AM".to_owned(),
                    mood: "good".to_owned(),
                    note: vec![
                        "some tag   another tag    yet another tag ".to_owned(),
                        "Note title".to_owned(),
                        "Note body".to_owned(),
                    ],
                    tags: vec![],
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

        let expected = Diary {
            day_entries: vec![
                DayEntry {
                    date: parse_date(&parsed.day_entries[0]).unwrap(),
                    moods: HashSet::from([Mood::new("rad")]),
                    tags: HashSet::new(),
                    note: "This is a note".to_owned(),
                },
                DayEntry {
                    date: parse_date(&parsed.day_entries[1]).unwrap(),
                    moods: HashSet::from([Mood::new("rad")]),
                    tags: HashSet::new(),
                    note: "This is a note²".to_owned(),
                },
                DayEntry {
                    date: parse_date(&parsed.day_entries[2]).unwrap(),
                    moods: HashSet::from([Mood::new("good")]),
                    tags: HashSet::from([
                        Tag::new("yet another tag"),
                        Tag::new("another tag"),
                        Tag::new("some tag"),
                    ]),
                    note: "Note title\nNote body".to_owned(),
                },
            ],
            moods: vec![
                MoodDetail {
                    name: "good".to_owned(),
                    wellbeing_value: Some(2),
                    icon_id: None,
                    category: None,
                },
                MoodDetail {
                    name: "rad".to_owned(),
                    wellbeing_value: Some(1),
                    icon_id: None,
                    category: None,
                },
            ],
            tags: vec![
                TagDetail {
                    name: "Tag that won't be matched".to_owned(),
                    icon_id: None,
                },
                TagDetail {
                    name: "another tag".to_owned(),
                    icon_id: None,
                },
                TagDetail {
                    name: "some tag".to_owned(),
                    icon_id: None,
                },
                TagDetail {
                    name: "yet another tag".to_owned(),
                    icon_id: None,
                },
            ],
        };

        let processed = Diary::try_from(parsed).unwrap();

        assert_eq!(processed, expected);
    }
}
