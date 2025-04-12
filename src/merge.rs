use crate::daylio::{CustomMood, Daylio, Tag};
use crate::{DayEntry, NUMBER_OF_PREDEFINED_MOODS};

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

    fn change_tag_id(day_entries: &mut [DayEntry], tag: &mut Tag, new_id: i64) {
        for entry in day_entries {
            for entry_tag in &mut entry.tags {
                if *entry_tag == tag.id {
                    *entry_tag = new_id;
                    break;
                }
            }
        }
        tag.id = new_id;
    }

    fn make_ids_distinct(&mut self, id_gen: &mut IdGenerator) {
        for mood in &mut self.custom_moods {
            Daylio::change_mood_id(&mut self.day_entries, mood, id_gen.next());
        }

        for tag in &mut self.tags {
            Daylio::change_tag_id(&mut self.day_entries, tag, id_gen.next());
        }
    }

    pub fn add_unique_moods(&mut self, mergee: &mut Daylio) {
        let is_mergeable = |mood1: &CustomMood, mood2: &CustomMood| {
            // Either both are the same predefined mood, or both are custom moods with the same name
            (mood1.predefined_name_id == mood2.predefined_name_id && mood1.predefined_name_id != -1)
                || (mood1.custom_name.to_uppercase() == mood2.custom_name.to_uppercase()
                    && mood1.predefined_name_id == -1
                    && mood2.predefined_name_id == -1)
        };

        let sort_by = |lhs: &CustomMood, rhs: &CustomMood| {
            lhs.predefined_name_id.cmp(&rhs.predefined_name_id).then(
                lhs.custom_name
                    .to_uppercase()
                    .cmp(&rhs.custom_name.to_uppercase()),
            )
        };

        self.custom_moods.sort_by(sort_by);
        mergee.custom_moods.sort_by(sort_by);

        let mut left_index = 0;
        let mut right_index = 0;

        while left_index < self.custom_moods.len() && right_index < mergee.custom_moods.len() {
            let self_mood = &mut self.custom_moods[left_index];
            let added_mood = &mut mergee.custom_moods[right_index];

            if is_mergeable(self_mood, added_mood) {
                Daylio::change_mood_id(&mut mergee.day_entries, added_mood, self_mood.id);
                right_index += 1;
            } else if sort_by(self_mood, added_mood) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                self.custom_moods.insert(left_index, added_mood.clone());
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining moods from mergee
        while right_index < mergee.custom_moods.len() {
            let added_mood = &mut mergee.custom_moods[right_index];
            self.custom_moods.push(added_mood.clone());
            right_index += 1;
        }
    }

    pub fn add_unique_tags(&mut self, mergee: &mut Daylio) {
        let is_mergeable = |tag1: &Tag, tag2: &Tag| tag1.name == tag2.name;

        let sort_by = |lhs: &Tag, rhs: &Tag| lhs.name.cmp(&rhs.name);
        self.tags.sort_by(sort_by);
        mergee.tags.sort_by(sort_by);

        let mut left_index = 0;
        let mut right_index = 0;

        while left_index < self.tags.len() && right_index < mergee.tags.len() {
            let self_tag = &mut self.tags[left_index];
            let added_tag = &mut mergee.tags[right_index];

            if is_mergeable(self_tag, added_tag) {
                Daylio::change_tag_id(&mut mergee.day_entries, added_tag, self_tag.id);
                right_index += 1;
            } else if sort_by(self_tag, added_tag) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                self.tags.insert(left_index, added_tag.clone());
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining tags from mergee
        while right_index < mergee.tags.len() {
            let added_tag = &mut mergee.tags[right_index];
            self.tags.push(added_tag.clone());
            right_index += 1;
        }
    }

    pub fn add_unique_entries(&mut self, mergee: &mut Daylio) {
        // Aggressive simplification of the note: only keep alphanumeric characters
        fn simplify_note_for_comparing(entry: &DayEntry) -> String {
            (entry.note_title.clone() + &entry.note)
                .chars()
                .filter(|c| c.is_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect()
        }

        fn is_mergeable(lhs: &mut DayEntry, rhs: &mut DayEntry) -> bool {
            // 1. They have the same mood
            let is_same_mood = lhs.mood == rhs.mood;

            // 2. They have almost the same date
            let timestamp_diff = (lhs.datetime - rhs.datetime).abs();
            let is_same_day = timestamp_diff < MILLISECONDS_IN_A_DAY;

            // 3. They have the same tags
            lhs.tags.sort_unstable();
            rhs.tags.sort_unstable();
            let is_same_tags = lhs.tags == rhs.tags;

            // 4. They have the same alphanumeric content
            let same_note = simplify_note_for_comparing(lhs) == simplify_note_for_comparing(rhs);

            is_same_day && is_same_mood && is_same_tags && same_note
        }

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

            if is_mergeable(self_entry, added_entry) {
                right_index += 1;
            } else if sort_by(self_entry, added_entry) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                self.day_entries.insert(left_index, added_entry.clone());
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining entries from mergee
        while right_index < mergee.day_entries.len() {
            let added_entry = &mut mergee.day_entries[right_index];
            self.day_entries.push(added_entry.clone());
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

        // predefined moods have to have the same id as the predefined name
        for mood in &mut self.custom_moods {
            if mood.predefined_name_id != -1 {
                Daylio::change_mood_id(&mut self.day_entries, mood, mood.predefined_name_id);
            }
        }

        let mut id_generator = IdGenerator::with_start(1, NUMBER_OF_PREDEFINED_MOODS + 1);

        // order is important, so we need to sort by mood_group_id and predefined comes first
        self.custom_moods
            .sort_by_key(|x| (x.mood_group_id, -x.predefined_name_id));
        for mood in &mut self.custom_moods {
            if mood.predefined_name_id == -1 {
                Daylio::change_mood_id(&mut self.day_entries, mood, id_generator.next());
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

        self.tags.sort_by_key(|x| x.created_at);
        let mut id_generator = IdGenerator::new(1);
        for (i, tag) in self.tags.iter_mut().enumerate() {
            Daylio::change_tag_id(&mut self.day_entries, tag, id_generator.next());
            tag.order = i as i64 + 1;
        }

        self.day_entries
            .sort_by_key(|x| (-x.datetime, -x.year, -x.month));
        let mut id_generator = IdGenerator::new(1);
        for entry in &mut self.day_entries {
            entry.id = id_generator.next();
        }
    }
}

/// Merges two daylio json files into one.
/// We assume the files have version 15, but this is not checked.
/// We keep everything from the first file, and add the new entries from the other files
#[must_use]
pub fn merge(mut reference: Daylio, mut mergee: Daylio) -> Daylio {
    const BIG_OFFSET: i64 = 1000;

    // first_pass: make sure we don't have any duplicates id
    let mut id_generator = IdGenerator::new(BIG_OFFSET);
    reference.make_ids_distinct(&mut id_generator);
    mergee.make_ids_distinct(&mut id_generator);

    reference.add_unique_moods(&mut mergee);
    reference.add_unique_tags(&mut mergee);
    reference.add_unique_entries(&mut mergee);

    reference.sanitize();

    // update metadata
    reference.metadata.number_of_entries = reference.day_entries.len() as i64;
    reference.metadata.number_of_photos += mergee.metadata.number_of_photos;
    reference.metadata.photos_size += mergee.metadata.photos_size;

    reference
}
