pub use crate::Daylio;
use crate::{
    daylio, daylio_predefined_mood_idx, daylio_predefined_mood_name,
    DaylioCustomMood, NUMBER_OF_PREDEFINED_MOODS,
};
use chrono::{DateTime, Datelike, NaiveDateTime, Timelike};
use color_eyre::eyre;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const NO_MOOD: LazyLock<DaylioCustomMood, fn() -> DaylioCustomMood> =
    LazyLock::new(|| DaylioCustomMood {
        id: 999_999,
        custom_name: String::from("Inconnu"),
        mood_group_id: 3,
        mood_group_order: 100,
        icon_id: 3,
        predefined_name_id: 0,
        state: 0,
        created_at: 0,
    });

#[derive(Debug, PartialEq, Clone, Default, Eq, Serialize, Deserialize)]
pub struct DayEntry {
    pub date: NaiveDateTime,
    pub moods: HashSet<Mood>,
    pub tags: HashSet<Tag>,
    pub note: String,
}

impl PartialOrd<Self> for DayEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.date.cmp(&other.date))
    }
}

impl Ord for DayEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date.cmp(&other.date)
    }
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Default, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag {
    #[serde(rename = "tag")]
    pub name: String,
}

impl Tag {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

#[derive(Eq, Hash, Debug, PartialEq, Clone, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Mood {
    #[serde(rename = "mood")]
    pub name: String,
}

impl Mood {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct TagDetail {
    pub name: String,
    pub icon_id: Option<i64>,
}

impl Ord for TagDetail {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct MoodDetail {
    pub name: String,
    pub icon_id: Option<i64>,
    pub wellbeing_value: u64,
    pub category: Option<String>,
}

impl Ord for MoodDetail {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wellbeing_value
            .cmp(&other.wellbeing_value)
            .then(self.name.cmp(&other.name))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diary {
    pub day_entries: Vec<DayEntry>,
    pub moods: Vec<MoodDetail>,
    pub tags: Vec<TagDetail>,
}

impl Diary {
    pub fn sorted(mut self) -> Self {
        self.day_entries.sort();
        self.moods.sort();
        self.tags.sort();
        self
    }
}

impl From<Daylio> for Diary {
    fn from(daylio: Daylio) -> Self {
        let moods: Vec<MoodDetail> = daylio
            .custom_moods
            .iter()
            .map(|mood| {
                let name = if !mood.custom_name.is_empty() {
                    mood.custom_name.clone()
                } else {
                    daylio_predefined_mood_name(mood.predefined_name_id)
                        .unwrap()
                        .into()
                };

                // This is obviously lossy, but we don't have more information in Daylio format
                let wellbeing_value = (mood.mood_group_id * 100 + mood.mood_group_order) as u64;

                MoodDetail {
                    name,
                    icon_id: Some(mood.icon_id),
                    wellbeing_value,
                    category: None,
                }
            })
            .collect();

        let tags: Vec<TagDetail> = daylio
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

        let day_entries: Vec<DayEntry> = daylio
            .day_entries
            .iter()
            .map(|entry| {
                let predefined_name = daylio_predefined_mood_name(entry.mood);
                let mood = {
                    let name = mood_map.get(&entry.mood).unwrap();
                    if let Some(predefined_name) = predefined_name
                        && name.custom_name.is_empty()
                    {
                        Mood::new(predefined_name)
                    } else {
                        Mood::new(name.custom_name.as_str())
                    }
                };

                DayEntry {
                    date: DateTime::from_timestamp_millis(entry.datetime)
                        .unwrap()
                        .naive_utc(),
                    moods: HashSet::from([mood]),
                    tags: entry
                        .tags
                        .iter()
                        .map(|tag_id| {
                            let tag = tag_map.get(tag_id).unwrap();
                            Tag::new(&tag.name)
                        })
                        .collect(),
                    note: if entry.note_title.is_empty() {
                        entry.note.clone()
                    } else {
                        format!("{}\n\n{}", &entry.note_title, &entry.note)
                    },
                }
            })
            .collect();

        Diary {
            day_entries,
            moods,
            tags,
        }
        .sorted()
    }
}

impl TryFrom<Diary> for Daylio {
    type Error = eyre::Error;
    fn try_from(diary: Diary) -> Result<Self, Self::Error> {
        let tags: Vec<daylio::DaylioTag> = diary
            .day_entries
            .iter()
            .flat_map(|entry| entry.tags.iter())
            .collect::<HashSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, tag)| {
                let detail = diary.tags.iter().find(|t| t.name == tag.name);

                daylio::DaylioTag {
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

        let max_mood_value = diary
            .moods
            .iter()
            .map(|m| m.wellbeing_value)
            .max()
            .unwrap_or(1);

        let all_moods: Vec<DaylioCustomMood> = diary
            .day_entries
            .iter()
            .flat_map(|entry| entry.moods.iter())
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

                // We want to group into 5 groups (1 to 5), best mood being 5
                let group_id = mood_detail
                    .wellbeing_value
                    .saturating_mul(5)
                    .checked_div(max_mood_value)
                    .unwrap_or(0);

                DaylioCustomMood {
                    id: predefined_name_id
                        .map_or(i as i64 + NUMBER_OF_PREDEFINED_MOODS as i64, |i| i as i64),
                    custom_name: if predefined_name_id.is_some() {
                        String::new()
                    } else {
                        mood.name.clone()
                    },
                    mood_group_id: group_id as i64,
                    mood_group_order: 0,
                    icon_id: mood_detail
                        .icon_id
                        .or(predefined_name_id.map(|i| i as i64))
                        .unwrap_or(i64::try_from(group_id).unwrap()),
                    predefined_name_id: predefined_name_id.map_or(-1, |x| x as i64),
                    state: 0,
                    created_at: 0,
                }
            })
            .chain(std::iter::once(NO_MOOD.clone()))
            .collect();

        let entries: Vec<daylio::DaylioDayEntry> = diary
            .day_entries
            .into_iter()
            .enumerate()
            .flat_map(|(i, entry)| {
                let entry_moods: Vec<Mood> = entry.moods.into_iter().collect();
                let main_entry = daylio::DaylioDayEntry {
                    id: i as i64,
                    minute: i64::from(entry.date.minute()),
                    hour: i64::from(entry.date.hour()),
                    day: i64::from(entry.date.day()),
                    month: i64::from(entry.date.month0()), // month is 0-indexed in Daylio
                    year: i64::from(entry.date.year()),
                    datetime: entry.date.and_utc().timestamp_millis(),
                    time_zone_offset: 0,
                    mood: if let Some(mood) = entry_moods.get(0) {
                        all_moods
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
                };

                // TODO: for now, we don't support multiple moods per entry in Daylio
                // One possible approach would be to create multiple entries for each mood,
                // but that's a lossy conversion.
                vec![main_entry]
            })
            .collect();

        let metadata = daylio::DaylioMetadata {
            number_of_entries: entries.len() as i64,
            ..Default::default()
        };

        let mut daylio = Daylio {
            tags,
            custom_moods: all_moods,
            day_entries: entries,
            metadata,
            ..Self::default()
        };
        daylio.sanitize();

        Ok(daylio)
    }
}
