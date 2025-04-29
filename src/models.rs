pub use crate::Daylio;
use crate::{
    CustomMood, NUMBER_OF_PREDEFINED_MOODS, daylio, daylio_predefined_mood_idx,
    daylio_predefined_mood_name,
};
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use color_eyre::eyre;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const NO_MOOD: LazyLock<CustomMood, fn() -> CustomMood> = LazyLock::new(|| CustomMood {
    id: 999_999,
    custom_name: String::from("Inconnu"),
    mood_group_id: 3,
    mood_group_order: 100,
    icon_id: 3,
    predefined_name_id: 0,
    state: 0,
    created_at: 0,
});

#[derive(Debug, PartialEq, Clone, Default, Eq)]
pub struct DayEntry {
    pub date: NaiveDateTime,
    pub mood: Option<Mood>,
    pub tags: HashSet<Tag>,
    pub note: String,
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default, Ord, PartialOrd)]
pub struct Tag {
    pub name: String,
}

impl Tag {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Ord, PartialOrd)]
pub struct Mood {
    pub name: String,
}

impl Mood {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TagDetail {
    pub name: String,
    pub icon_id: Option<i64>,
}

#[derive(Debug, Default, Clone)]
pub struct MoodDetail {
    pub name: String,
    pub icon_id: Option<i64>,
    pub group: u8,
}

#[derive(Debug, Default, Clone)]
pub struct Diary {
    pub day_entries: Vec<DayEntry>,
    pub moods: Vec<MoodDetail>,
    pub tags: Vec<TagDetail>,
}

impl From<Daylio> for Diary {
    fn from(daylio: Daylio) -> Self {
        let moods = daylio
            .custom_moods
            .iter()
            .map(|mood| MoodDetail {
                name: mood.custom_name.clone(),
                icon_id: Some(mood.icon_id),
                group: u8::try_from(mood.mood_group_id).unwrap(),
            })
            .collect();

        let tags = daylio
            .tags
            .iter()
            .map(|tag| TagDetail {
                name: tag.name.clone(),
                icon_id: Some(tag.icon),
            })
            .collect();

        let mood_map = daylio
            .custom_moods
            .iter()
            .map(|mood| (mood.id, mood))
            .collect::<HashMap<_, _>>();

        let tag_map = daylio
            .tags
            .iter()
            .map(|tag| (tag.id, tag))
            .collect::<HashMap<_, _>>();

        let day_entries = daylio
            .day_entries
            .iter()
            .map(|entry| {
                let predefined_name = daylio_predefined_mood_name(entry.mood);
                let mood = if let Some(predefined_name) = predefined_name {
                    Mood::new(predefined_name)
                } else {
                    let name = mood_map.get(&entry.mood).unwrap();
                    Mood::new(&name.custom_name)
                };

                DayEntry {
                    date: DateTime::from_timestamp_millis(entry.datetime)
                        .unwrap()
                        .naive_utc(),
                    mood: Some(mood),
                    tags: entry
                        .tags
                        .iter()
                        .map(|tag_id| {
                            let tag = tag_map.get(tag_id).unwrap();
                            Tag::new(&tag.name)
                        })
                        .collect(),
                    note: entry.note_title.clone() + &entry.note,
                }
            })
            .collect();

        Diary {
            day_entries,
            moods,
            tags,
        }
    }
}

impl TryFrom<Diary> for Daylio {
    type Error = eyre::Error;
    fn try_from(diary: Diary) -> Result<Self, Self::Error> {
        let tags: Vec<daylio::Tag> = diary
            .day_entries
            .iter()
            .flat_map(|entry| entry.tags.iter())
            .collect::<HashSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, tag)| {
                let detail = diary.tags.iter().find(|t| t.name == tag.name);

                daylio::Tag {
                    id: i as i64,
                    name: tag.name.clone(),
                    created_at: 0,
                    icon: detail.and_then(|t| t.icon_id).unwrap_or(0),
                    order: 0,
                    state: 0,
                    id_tag_group: 0,
                }
            })
            .collect();

        let moods: Vec<daylio::CustomMood> = diary
            .day_entries
            .iter()
            .filter_map(|entry| entry.mood.as_ref())
            .collect::<HashSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, mood)| {
                let mood_detail = diary
                    .moods
                    .iter()
                    .find(|m| m.name == mood.name)
                    .expect("Mood not found in diary");

                let predefined_name_id = daylio_predefined_mood_idx(&mood.name);
                daylio::CustomMood {
                    id: predefined_name_id
                        .map_or(i as i64 + NUMBER_OF_PREDEFINED_MOODS as i64, |i| i as i64),
                    custom_name: if predefined_name_id.is_some() {
                        String::new()
                    } else {
                        mood.name.clone()
                    },
                    mood_group_id: i64::from(mood_detail.group),
                    mood_group_order: 0,
                    icon_id: mood_detail
                        .icon_id
                        .or(predefined_name_id.map(|i| i as i64))
                        .unwrap_or(i64::from(mood_detail.group)),
                    predefined_name_id: predefined_name_id.map_or(-1, |x| x as i64),
                    state: 0,
                    created_at: 0,
                }
            })
            .chain(std::iter::once(NO_MOOD.clone()))
            .collect();

        let entries: Vec<daylio::DayEntry> = diary
            .day_entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| daylio::DayEntry {
                id: i as i64,
                minute: i64::from(entry.date.minute()),
                hour: i64::from(entry.date.hour()),
                day: i64::from(entry.date.day()),
                month: i64::from(entry.date.month0()), // month is 0-indexed in Daylio
                year: i64::from(entry.date.year()),
                datetime: entry.date.and_utc().timestamp_millis(),
                time_zone_offset: 0,
                mood: if let Some(mood) = entry.mood {
                    moods
                        .iter()
                        .find(|m| m.custom_name == mood.name)
                        .unwrap()
                        .id
                } else {
                    NO_MOOD.id
                },
                tags: entry
                    .tags
                    .iter()
                    .map(|tag| tags.iter().find(|t| t.name == tag.name).unwrap().id)
                    .collect(),
                note: entry.note,
                note_title: String::new(),
                assets: vec![],
            })
            .collect();

        let metadata = daylio::Metadata {
            number_of_entries: entries.len() as i64,
            ..Default::default()
        };

        let mut daylio = Daylio {
            tags,
            custom_moods: moods,
            day_entries: entries,
            metadata,
            ..Self::default()
        };
        daylio.sanitize();

        Ok(daylio)
    }
}
