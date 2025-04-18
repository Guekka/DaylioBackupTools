use crate::DayEntry;
use crate::daylio::{CustomMood, Daylio, Tag};
use color_eyre::eyre::Context;

#[derive(Clone, Copy)]
struct IdGenerator {
    offset: i64,
    current: i64,
}

impl IdGenerator {
    fn new(offset: i64) -> Self {
        Self::with_start(offset, offset)
    }

    fn with_start(offset: i64, start: i64) -> Self {
        Self {
            offset,
            current: start - offset,
        }
    }

    fn next(&mut self) -> i64 {
        self.current += self.offset;
        self.current
    }
}

const MILLISECONDS_IN_A_DAY: i64 = 24 * 60 * 60 * 1000;

impl Daylio {
    fn change_mood_id(day_entries: &mut [DayEntry], mood: &mut CustomMood, new_id: i64) {
        for entry in day_entries {
            if entry.mood == mood.id {
                entry.mood = new_id;
            }
        }
        mood.id = new_id;
    }

    /// Aggressive simplification of the note: only keep alphanumeric characters
    fn simplify_note_for_comparing(entry: &DayEntry) -> String {
        (entry.note_title.clone() + &entry.note)
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    fn add_entry(&mut self, mut entry: DayEntry, other: &Daylio, idx: usize) {
        entry.id = self.day_entries.len() as i64 + 1;

        let is_duplicate_mood = |mood1: &CustomMood, mood2: &CustomMood| {
            let same_predefined = mood1.predefined_name_id == mood2.predefined_name_id
                && mood1.predefined_name_id != -1;

            let same_custom = mood1.custom_name.to_uppercase() == mood2.custom_name.to_uppercase()
                && !mood1.custom_name.is_empty();

            same_custom || same_predefined
        };

        let added_mood = &mut other
            .custom_moods
            .iter()
            .find(|mood| entry.mood == mood.id)
            .expect("Added mood not found")
            .clone();

        let corresponding_self_mood = self
            .custom_moods
            .iter_mut()
            .find(|mood| is_duplicate_mood(mood, added_mood));

        if let Some(self_mood) = corresponding_self_mood {
            // We keep the one from the reference file
            entry.mood = self_mood.id;
        } else {
            let new_id = self
                .custom_moods
                .iter()
                .map(|mood| mood.id)
                .max()
                .unwrap_or(0)
                + 1;
            added_mood.id = new_id;
            entry.mood = new_id;
            self.custom_moods.push(added_mood.clone());
        }

        let is_duplicate_tag =
            |tag1: &Tag, tag2: &Tag| tag1.name.to_uppercase() == tag2.name.to_uppercase();

        for entry_tag in &mut entry.tags {
            let added_tag = &mut other
                .tags
                .iter()
                .find(|tag| tag.id == *entry_tag)
                .expect("Added tag not found")
                .clone();

            let corresponding_self_tag = self
                .tags
                .iter_mut()
                .find(|tag| is_duplicate_tag(tag, added_tag));

            if let Some(self_tag) = corresponding_self_tag {
                *entry_tag = self_tag.id;
            } else {
                let new_id = self.tags.iter().map(|tag| tag.id).max().unwrap_or(0) + 1;
                added_tag.id = new_id;
                *entry_tag = new_id;
                self.tags.push(added_tag.clone());
            }
        }

        // Add the entry to the reference file
        self.day_entries.insert(idx, entry.clone());
    }

    pub fn add_unique_entries(&mut self, mergee: &mut Daylio) {
        let sort_by = |lhs: &DayEntry, rhs: &DayEntry| {
            lhs.datetime
                .cmp(&rhs.datetime)
                .then(lhs.mood.cmp(&rhs.mood))
                .then(lhs.tags.cmp(&rhs.tags))
        };

        self.day_entries.sort_by(sort_by);
        mergee.day_entries.sort_by(sort_by);

        let mut left_index = 0;
        let mut right_index = 0;

        while left_index < self.day_entries.len() && right_index < mergee.day_entries.len() {
            let self_entry = &mut self.day_entries[left_index];
            let added_entry = &mut mergee.day_entries[right_index];

            let timestamp_diff = (self_entry.datetime - added_entry.datetime).abs();
            let same_day = timestamp_diff < MILLISECONDS_IN_A_DAY;

            let same_note = Self::simplify_note_for_comparing(self_entry)
                == Self::simplify_note_for_comparing(added_entry);

            if same_day && same_note {
                // We keep the one from the reference file
                right_index += 1;
            } else if sort_by(self_entry, added_entry) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                self.add_entry(added_entry.clone(), mergee, left_index);
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining entries from mergee
        while right_index < mergee.day_entries.len() {
            let added_entry = &mut mergee.day_entries[right_index];
            self.add_entry(added_entry.clone(), mergee, left_index);
            right_index += 1;
        }
    }

    pub fn sanitize(&mut self) {
        // fix: sometimes custom moods have a custom
        // name and a predefined name
        // we keep custom name and remove predefined name
        for mood in &mut self.custom_moods {
            if mood.predefined_name_id != -1 && !mood.custom_name.is_empty() {
                mood.predefined_name_id = -1;
            }
        }

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
        let mut id_generator = IdGenerator::new(100_000);
        for mood in &mut self.custom_moods {
            Self::change_mood_id(&mut self.day_entries, mood, id_generator.next());
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
        let mut id_generator = IdGenerator::new(1);
        for entry in &mut self.day_entries {
            entry.id = id_generator.next();
        }
    }
}

/// Merges two daylio files into one.
/// We assume the files have version 15, but this is not checked.
/// We keep everything from the first file, and add the new entries from the other files
pub fn merge(mut reference: Daylio, mut mergee: Daylio) -> color_eyre::Result<Daylio> {
    reference
        .check_soundness()
        .context("Reference file is not sound. Please check the file and try again.")?;

    mergee
        .check_soundness()
        .context("Mergee file is not sound. Please check the file and try again.")?;

    reference.add_unique_entries(&mut mergee);

    reference.sanitize();

    reference
        .check_soundness()
        .context("Merged file is not sound. This is a bug. Please report it.")?;

    // update metadata
    reference.metadata.number_of_entries = reference.day_entries.len() as i64;
    reference.metadata.number_of_photos += mergee.metadata.number_of_photos;
    reference.metadata.photos_size += mergee.metadata.photos_size;

    Ok(reference)
}
