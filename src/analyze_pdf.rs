//! This module interprets the parsed PDF data into a Daylio struct.
#![allow(dead_code)]
use crate::parse_pdf::StatLine;

#[derive(Clone, Debug, PartialEq)]
struct Mood {
    name: String,
    count: u32,
}

impl From<StatLine> for Mood {
    fn from(stat: StatLine) -> Self {
        Self {
            name: stat.name,
            count: stat.count,
        }
    }
}

impl Mood {
    fn new(name: String, count: u32) -> Self {
        Self { name, count }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Tag {
    name: String,
    count: u32,
}

impl From<StatLine> for Tag {
    fn from(stat: StatLine) -> Self {
        Self {
            name: stat.name,
            count: stat.count,
        }
    }
}

impl Tag {
    fn new(name: String, count: u32) -> Self {
        Self { name, count }
    }
}

const MOOD_CATEGORY_COUNT: usize = 5;
#[derive(Clone, Debug, PartialEq)]
struct Stats {
    moods: [Vec<Mood>; MOOD_CATEGORY_COUNT],
    tags: Vec<Tag>,
}

/// The PDF extractor reads the file left to right
/// But the stats page is top to bottom, so we need to transpose the data
/// There are 3 columns, so we need to split the data into 3 chunks
/// However, the first column might not have the same number of rows as the second one, even
/// if the third one is not empty
/// What we know, however, is that:
/// - the first data is the mood
/// - each mood category is sorted by descending count
/// - after the mood, there are tags (sorted by descending count)
/// So we only have to extract the first column, split it into moods and tags, and then
/// sort the tags by descending count
fn analyze_stat_lines(lines: Vec<StatLine>) -> Stats {
    let first_column = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 3 == 0)
        .map(|(_, tag)| tag.clone())
        .collect::<Vec<StatLine>>();

    // array of vec
    let mut moods: Vec<Vec<Mood>> = Vec::with_capacity(MOOD_CATEGORY_COUNT);

    moods.push(vec![first_column[0].clone().into()]);
    for i in 1..first_column.len() {
        let prev_mood = &first_column[i - 1];
        let cur_mood: Mood = first_column[i].clone().into();
        if prev_mood.count >= cur_mood.count {
            moods.last_mut().unwrap().push(cur_mood);
        } else {
            moods.push(vec![cur_mood]);
        }
    }

    // some tags might be part of the first column. Let's remove "moods" from the last category
    // that are not sorted
    let last_category = moods.last_mut().unwrap();
    if last_category.len() > 1 {
        // find first not sorted
        for i in (1..last_category.len()).rev() {
            if last_category[i - 1].count < last_category[i].count {
                println!("{} {}", last_category[i - 1].count, last_category[i].count);
                last_category.truncate(i);
                break;
            }
        }
    }

    // there's a catch though
    // two adjacent moods from different categories could have a sorted count
    // we know there's at least one mood in each category, so we're going to enforce that

    fn move_last_to_next_category(moods: &mut Vec<Vec<Mood>>, category_idx: usize) {
        if category_idx + 2 > moods.len() {
            moods.push(vec![]);
        }

        let last = moods[category_idx].pop().unwrap();
        moods[category_idx + 1].push(last);
    }

    while moods.len() < MOOD_CATEGORY_COUNT {
        // find the last category that has more than one mood
        let last_category_with_more_than_one_mood = moods
            .iter()
            .enumerate()
            .rev()
            .find(|(_, category)| category.len() > 1)
            .unwrap()
            .0;

        move_last_to_next_category(&mut moods, last_category_with_more_than_one_mood);
    }

    // alright, done with the moods! now we can sort the tags
    // let's start by removing the moods from the tags
    let mut tags = lines;

    let moods_count = moods.iter().flatten().count();
    tags.drain(0..moods_count);
    tags.sort_unstable_by_key(|tag| tag.count);

    let tags = tags.into_iter().map(Into::into).collect();

    Stats {
        moods: moods.try_into().unwrap(),
        tags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_pdf::tests::expected_parsed_tags;

    #[ignore] //
    #[test]
    fn test_analyze_stats() {
        let input = expected_parsed_tags();

        let expected = Stats {
            moods: [
                vec![
                    Mood::new("rad".to_owned(), 15),
                    Mood::new("Mood 0 KWY".to_owned(), 5),
                ],
                vec![
                    Mood::new("good".to_owned(), 20),
                    Mood::new("Mood 1 QBL".to_owned(), 13),
                ],
                vec![
                    Mood::new("meh".to_owned(), 1),
                    Mood::new("Mood 2 VUP".to_owned(), 8),
                ],
                vec![Mood::new("bad".to_owned(), 2)],
                vec![Mood::new("bad".to_owned(), 2)],
            ],
            tags: vec![
                Tag::new("Tag 5 IGN".to_owned(), 14),
                Tag::new("Tag 4 HBK".to_owned(), 10),
                Tag::new("Tag 21 NUD".to_owned(), 9),
                Tag::new("Tag 11 XRB".to_owned(), 8),
                Tag::new("Tag 6 AUG".to_owned(), 6),
                Tag::new("Tag 10 OKU".to_owned(), 6),
                Tag::new("Tag 23 CLN".to_owned(), 5),
                Tag::new("Tag 2 NWR".to_owned(), 4),
                Tag::new("Tag 12 LRD".to_owned(), 3),
                Tag::new("Tag 0 AHY".to_owned(), 2),
                Tag::new("Tag 8 WNA".to_owned(), 2),
                Tag::new("Tag 14 NEU".to_owned(), 2),
                Tag::new("Tag 9 MAS".to_owned(), 1),
                Tag::new("Tag 16 QUG".to_owned(), 1),
                Tag::new("Tag 22 ITV".to_owned(), 1),
                Tag::new("Tag 24 KVI".to_owned(), 1),
                Tag::new("Tag 25 CGQ".to_owned(), 1),
                Tag::new("Tag 33 IQP".to_owned(), 1),
            ],
        };

        let parsed = analyze_stat_lines(input);

        assert_eq!(parsed, expected);
    }
}
