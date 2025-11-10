use color_eyre::eyre;
use core::default::Default;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

pub const NUMBER_OF_PREDEFINED_MOODS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Daylio {
    pub version: i64,
    pub is_reminder_on: bool,
    pub custom_moods: Vec<DaylioCustomMood>,
    pub tags: Vec<DaylioTag>,
    pub day_entries: Vec<DaylioDayEntry>,
    pub achievements: Vec<Achievement>,
    pub days_in_row_longest_chain: i64,
    pub goals: Vec<Value>,
    pub prefs: Vec<Pref>,
    #[serde(rename = "tag_groups")]
    pub tag_groups: Vec<TagGroup>,
    pub metadata: DaylioMetadata,
    pub mood_icons_pack_id: i64,
    pub preferred_mood_icons_ids_for_mood_ids_for_icons_pack: Value,
    pub assets: Vec<Value>,
    pub goal_entries: Vec<Value>,
    pub goal_success_weeks: Vec<Value>,
    pub reminders: Vec<Reminder>,
    pub writing_templates: Vec<WritingTemplate>,
    pub mood_icons_default_free_pack_id: i64,
}

impl Daylio {
    pub(crate) fn check_soundness(&self) -> eyre::Result<()> {
        for entry in &self.day_entries {
            if !self.custom_moods.iter().any(|mood| mood.id == entry.mood) {
                eyre::bail!("Invalid mood id {} in entry {:?}", entry.mood, entry);
            }

            for tag in &entry.tags {
                if !self.tags.iter().any(|t| t.id == *tag) {
                    eyre::bail!("Invalid tag id {} in entry {:?}", tag, entry);
                }
            }

            for i in 1..=NUMBER_OF_PREDEFINED_MOODS as i64 {
                if !self
                    .custom_moods
                    .iter()
                    .any(|mood| mood.predefined_name_id == i)
                {
                    eyre::bail!("Missing predefined mood {}", i);
                }
            }
        }

        Ok(())
    }

    fn change_mood_id(
        day_entries: &mut [DaylioDayEntry],
        mood: &mut DaylioCustomMood,
        new_id: i64,
    ) {
        for entry in day_entries {
            if entry.mood == mood.id {
                entry.mood = new_id;
            }
        }
        mood.id = new_id;
    }

    pub fn sanitize(&mut self) {
        const BIG_OFFSET: i64 = 100_000;

        // make sure all default moods are present
        for default_mood in Daylio::default().custom_moods {
            if !self
                .custom_moods
                .iter()
                .any(|mood| mood.predefined_name_id == default_mood.predefined_name_id)
            {
                self.custom_moods.push(default_mood);
            }
        }

        // first pass on moods, to avoid collisions when changing ids
        for (i, mood) in self.custom_moods.iter_mut().enumerate() {
            Self::change_mood_id(&mut self.day_entries, mood, i as i64 * BIG_OFFSET);
        }

        // order is important, so we need to sort by mood_group_id and predefined comes first
        self.custom_moods
            .sort_by_key(|x| (x.mood_group_id, -x.predefined_name_id));

        // predefined moods have to have the same id as the predefined name
        for mood in &mut self.custom_moods {
            if mood.predefined_name_id != -1 {
                Daylio::change_mood_id(&mut self.day_entries, mood, mood.predefined_name_id);
            }
        }

        // each mood group has an order, so we need to update it
        for i in 0..self.custom_moods.len() {
            if i == 0
                || self.custom_moods[i].mood_group_id != self.custom_moods[i - 1].mood_group_id
            {
                self.custom_moods[i].mood_group_order = 0;
            } else {
                self.custom_moods[i].mood_group_order =
                    self.custom_moods[i - 1].mood_group_order + 1;
            }
        }

        // make sure entries are sorted
        self.day_entries
            .sort_by_key(|x| (-x.datetime, -x.year, -x.month));
        for (i, entry) in self.day_entries.iter_mut().enumerate() {
            entry.id = i as i64;
        }
    }
}

#[must_use]
pub fn daylio_predefined_mood_idx(custom_name: &str) -> Option<u64> {
    match custom_name.to_lowercase().as_ref() {
        "super" | "rad" => Some(1),
        "bien" | "good" => Some(2),
        "mouais" | "meh" => Some(3),
        "mauvais" | "bad" => Some(4),
        "horrible" | "awful" => Some(5),
        _ => None,
    }
}

#[must_use]
pub fn daylio_predefined_mood_name(id: i64) -> Option<&'static str> {
    match id {
        1 => Some("super"),
        2 => Some("bien"),
        3 => Some("mouais"),
        4 => Some("mauvais"),
        5 => Some("horrible"),
        _ => None,
    }
}

impl Default for Daylio {
    fn default() -> Self {
        let moods = (1..=NUMBER_OF_PREDEFINED_MOODS as i64)
            .map(|i| DaylioCustomMood {
                id: i,
                icon_id: i,
                predefined_name_id: i,
                mood_group_id: i,
                ..Default::default()
            })
            .collect();

        Self {
            version: 15,
            is_reminder_on: Default::default(),
            custom_moods: moods,
            tags: vec![],
            day_entries: vec![],
            achievements: vec![],
            days_in_row_longest_chain: 0,
            goals: vec![],
            prefs: vec![
                Pref {
                    key: "BACKUP_REMINDER_DONT_SHOW_AGAIN".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 0.into(),
                },
                Pref {
                    key: "LAST_DAYS_IN_ROWS_NUMBER".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 0.into(),
                },
                Pref {
                    key: "DAYS_IN_ROW_LONGEST_CHAIN".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 0.into(),
                },
                Pref {
                    key: "LAST_ENTRY_CREATION_TIME".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 0.into(),
                },
                Pref {
                    key: "COLOR_PALETTE_DEFAULT_CODE".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 1.into(),
                },
                Pref {
                    key: "PREDEFINED_MOODS_VARIANT".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 1.into(),
                },
                Pref {
                    key: "ONBOARDING_USER_PROPERTY".to_owned(),
                    pref_name: "default".to_owned(),
                    value: "finished".into(),
                },
                Pref {
                    key: "WAS_EMOJI_SCREEN_VISITED".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 0.into(),
                },
                Pref {
                    key: "PIN_LOCK_STATE".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 2.into(),
                },
                Pref {
                    key: "ARE_MEMORIES_VISIBLE_TO_USER".to_owned(),
                    pref_name: "default".to_owned(),
                    value: 1.into(),
                },
            ],
            tag_groups: vec![TagGroup {
                id: 1,
                name: "Default".to_owned(),
                is_expanded: true,
                order: 1,
            }],
            metadata: DaylioMetadata::default(),
            mood_icons_pack_id: 1,
            preferred_mood_icons_ids_for_mood_ids_for_icons_pack: serde_json::json!(
                {
                    "1": {
                        "6": 6,
                        "7": 14,
                        "8": 14,
                    }
                }
            ),
            assets: vec![],
            goal_entries: vec![],
            goal_success_weeks: vec![],
            reminders: vec![],
            writing_templates: vec![],
            mood_icons_default_free_pack_id: 1,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaylioCustomMood {
    pub id: i64,
    #[serde(rename = "custom_name")]
    pub custom_name: String,
    #[serde(rename = "mood_group_id")]
    pub mood_group_id: i64,
    #[serde(rename = "mood_group_order")]
    pub mood_group_order: i64,
    #[serde(rename = "icon_id")]
    pub icon_id: i64,
    #[serde(rename = "predefined_name_id")]
    pub predefined_name_id: i64,
    pub state: i64,
    pub created_at: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaylioTag {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
    pub icon: i64,
    pub order: i64,
    pub state: i64,
    #[serde(rename = "id_tag_group")]
    pub id_tag_group: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaylioDayEntry {
    pub id: i64,
    pub minute: i64,
    pub hour: i64,
    pub day: i64,
    pub month: i64,
    pub year: i64,
    pub datetime: i64,
    pub time_zone_offset: i64,
    pub mood: i64,
    pub note: String,
    #[serde(rename = "note_title")]
    pub note_title: String,
    pub tags: Vec<i64>,
    pub assets: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub name: String,
    #[serde(
        rename = "AC_FIRST_ENTRY_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_first_entry_seen: Option<bool>,
    #[serde(
        rename = "AC_FIRST_ENTRY_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_first_entry_unlocked_at: Option<i64>,
    #[serde(rename = "AC_ENTRIES_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_entries_seen: Option<bool>,
    #[serde(
        rename = "AC_ENTRIES_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_current_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_current_value: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_BONUS_LVL_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_bonus_lvl_seen: Option<bool>,
    #[serde(
        rename = "AC_ENTRIES_BONUS_LVL_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_bonus_lvl_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_BONUS_LVL_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_bonus_lvl_current_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_BONUS_LVL_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_bonus_lvl_current_value: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_BONUS_LVL_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_bonus_lvl_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_MILLENNIUMS_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_millenniums_seen: Option<bool>,
    #[serde(
        rename = "AC_ENTRIES_MILLENNIUMS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_millenniums_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_MILLENNIUMS_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_millenniums_current_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_MILLENNIUMS_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_millenniums_current_value: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_MILLENNIUMS_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_millenniums_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_ETERNITY_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_eternity_seen: Option<bool>,
    #[serde(
        rename = "AC_ENTRIES_ETERNITY_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_eternity_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_ETERNITY_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_eternity_current_level: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_ETERNITY_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_eternity_current_value: Option<i64>,
    #[serde(
        rename = "AC_ENTRIES_ETERNITY_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_entries_eternity_last_seen_level: Option<i64>,
    #[serde(rename = "AC_STREAK_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_STREAK_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_current_level: Option<i64>,
    #[serde(
        rename = "AC_STREAK_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_current_value: Option<i64>,
    #[serde(
        rename = "AC_STREAK_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_MEGA_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mega_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_MEGA_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mega_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MEGA_STREAK_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mega_streak_current_level: Option<i64>,
    #[serde(
        rename = "AC_MEGA_STREAK_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mega_streak_current_value: Option<i64>,
    #[serde(
        rename = "AC_MEGA_STREAK_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mega_streak_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_EPIC_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_epic_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_EPIC_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_epic_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_EPIC_STREAK_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_epic_streak_current_level: Option<i64>,
    #[serde(
        rename = "AC_EPIC_STREAK_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_epic_streak_current_value: Option<i64>,
    #[serde(
        rename = "AC_EPIC_STREAK_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_epic_streak_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_MYTHICAL_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mythical_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_MYTHICAL_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mythical_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MYTHICAL_STREAK_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mythical_streak_current_level: Option<i64>,
    #[serde(
        rename = "AC_MYTHICAL_STREAK_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mythical_streak_current_value: Option<i64>,
    #[serde(
        rename = "AC_MYTHICAL_STREAK_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mythical_streak_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_STREAK_BONUS_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_bonus_seen: Option<bool>,
    #[serde(
        rename = "AC_STREAK_BONUS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_streak_bonus_unlocked_at: Option<i64>,
    #[serde(rename = "AC_TAGS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_tags_seen: Option<bool>,
    #[serde(
        rename = "AC_TAGS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_tags_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_TAGS_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_tags_current_level: Option<i64>,
    #[serde(
        rename = "AC_TAGS_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_tags_current_value: Option<i64>,
    #[serde(
        rename = "AC_TAGS_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_tags_last_seen_level: Option<i64>,
    #[serde(rename = "AC_MOODS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_moods_seen: Option<bool>,
    #[serde(
        rename = "AC_MOODS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MOODS_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_current_level: Option<i64>,
    #[serde(
        rename = "AC_MOODS_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_current_value: Option<i64>,
    #[serde(
        rename = "AC_MOODS_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_last_seen_level: Option<i64>,
    #[serde(
        rename = "AC_GOALS_DEDICATED_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_goals_dedicated_seen: Option<bool>,
    #[serde(
        rename = "AC_GOALS_DEDICATED_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_goals_dedicated_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_GOALS_DEDICATED_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_goals_dedicated_current_level: Option<i64>,
    #[serde(
        rename = "AC_GOALS_DEDICATED_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_goals_dedicated_current_value: Option<i64>,
    #[serde(
        rename = "AC_GOALS_DEDICATED_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_goals_dedicated_last_seen_level: Option<i64>,
    #[serde(rename = "AC_PAPARAZZI_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_paparazzi_seen: Option<bool>,
    #[serde(
        rename = "AC_PAPARAZZI_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_paparazzi_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_PAPARAZZI_CURRENT_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_paparazzi_current_level: Option<i64>,
    #[serde(
        rename = "AC_PAPARAZZI_CURRENT_VALUE",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_paparazzi_current_value: Option<i64>,
    #[serde(
        rename = "AC_PAPARAZZI_LAST_SEEN_LEVEL",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_paparazzi_last_seen_level: Option<i64>,
    #[serde(rename = "AC_COLORS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_colors_seen: Option<bool>,
    #[serde(
        rename = "AC_COLORS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_colors_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MULTIPLE_ENTRIES_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_multiple_entries_seen: Option<bool>,
    #[serde(
        rename = "AC_MULTIPLE_ENTRIES_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_multiple_entries_unlocked_at: Option<i64>,
    #[serde(rename = "AC_GROUPS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_groups_seen: Option<bool>,
    #[serde(
        rename = "AC_GROUPS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_groups_unlocked_at: Option<i64>,
    #[serde(rename = "AC_STYLE_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_style_seen: Option<bool>,
    #[serde(
        rename = "AC_STYLE_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_style_unlocked_at: Option<i64>,
    #[serde(rename = "AC_SMART_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_smart_seen: Option<bool>,
    #[serde(
        rename = "AC_SMART_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_smart_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_AUTO_BACKUP_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_auto_backup_seen: Option<bool>,
    #[serde(
        rename = "AC_AUTO_BACKUP_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_auto_backup_unlocked_at: Option<i64>,
    #[serde(rename = "AC_PREMIUM_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_premium_seen: Option<bool>,
    #[serde(
        rename = "AC_PREMIUM_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_premium_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_ROLLERCOASTER_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_rollercoaster_seen: Option<bool>,
    #[serde(
        rename = "AC_ROLLERCOASTER_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_rollercoaster_unlocked_at: Option<i64>,
    #[serde(rename = "AC_PIN_CODE_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_pin_code_seen: Option<bool>,
    #[serde(
        rename = "AC_PIN_CODE_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_pin_code_unlocked_at: Option<i64>,
    #[serde(rename = "AC_NO_BACKUP_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_no_backup_seen: Option<bool>,
    #[serde(
        rename = "AC_NO_BACKUP_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_no_backup_unlocked_at: Option<i64>,
    #[serde(rename = "AC_MEH_DAYS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_meh_days_seen: Option<bool>,
    #[serde(
        rename = "AC_MEH_DAYS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_meh_days_unlocked_at: Option<i64>,
    #[serde(rename = "AC_GOOD_DAYS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_good_days_seen: Option<bool>,
    #[serde(
        rename = "AC_GOOD_DAYS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_good_days_unlocked_at: Option<i64>,
    #[serde(rename = "AC_RAD_DAYS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_rad_days_seen: Option<bool>,
    #[serde(
        rename = "AC_RAD_DAYS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_rad_days_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MOODS_BONUS_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_bonus_seen: Option<bool>,
    #[serde(
        rename = "AC_MOODS_BONUS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_moods_bonus_unlocked_at: Option<i64>,
    #[serde(rename = "AC_TAGS_BONUS_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_tags_bonus_seen: Option<bool>,
    #[serde(
        rename = "AC_TAGS_BONUS_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_tags_bonus_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_LUCKY_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_lucky_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_LUCKY_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_lucky_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_CRYPTIC_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_cryptic_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_CRYPTIC_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_cryptic_streak_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_MYSTERIOUS_STREAK_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mysterious_streak_seen: Option<bool>,
    #[serde(
        rename = "AC_MYSTERIOUS_STREAK_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_mysterious_streak_unlocked_at: Option<i64>,
    #[serde(rename = "AC_SAY_CHEESE_SEEN", skip_serializing_if = "Option::is_none")]
    pub ac_say_cheese_seen: Option<bool>,
    #[serde(
        rename = "AC_SAY_CHEESE_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_say_cheese_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2022_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2022_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2022_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2022_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2021_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2021_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2021_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2021_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2020_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2020_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2020_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2020_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2019_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2019_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2019_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2019_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2018_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2018_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2018_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2018_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2017_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2017_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2017_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2017_unlocked_at: Option<i64>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2016_SEEN",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2016_seen: Option<bool>,
    #[serde(
        rename = "AC_YEARLY_REPORT_2016_UNLOCKED_AT",
        skip_serializing_if = "Option::is_none"
    )]
    pub ac_yearly_report_2016_unlocked_at: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pref {
    pub key: String,
    #[serde(rename = "pref_name")]
    pub pref_name: String,
    pub value: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroup {
    pub id: i64,
    pub name: String,
    #[serde(rename = "is_expanded")]
    pub is_expanded: bool,
    pub order: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaylioMetadata {
    #[serde(rename = "number_of_entries")]
    pub number_of_entries: i64,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    #[serde(rename = "is_auto_backup")]
    pub is_auto_backup: bool,
    pub platform: String,
    #[serde(rename = "android_version")]
    pub android_version: i64,
    #[serde(rename = "number_of_photos")]
    pub number_of_photos: i64,
    #[serde(rename = "photos_size")]
    pub photos_size: i64,
}

impl Default for DaylioMetadata {
    fn default() -> Self {
        DaylioMetadata {
            number_of_entries: 0,
            created_at: 0,
            is_auto_backup: false,
            platform: "android".to_owned(),
            android_version: 15,
            number_of_photos: 0,
            photos_size: 0,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: i64,
    pub hour: i64,
    pub minute: i64,
    pub state: i64,
    #[serde(rename = "custom_text_enabled")]
    pub custom_text_enabled: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingTemplate {
    pub id: i64,
    pub order: i64,
    #[serde(rename = "predefined_template_id")]
    pub predefined_template_id: i64,
    pub title: String,
    pub body: String,
}
