use crate::daylio::{CustomMood, Daylio, Tag};
use crate::DayEntry;

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

impl PartialEq for CustomMood {
    fn eq(&self, other: &Self) -> bool {
        self.custom_name.to_lowercase() == other.custom_name.to_lowercase()
            && self.icon_id == other.icon_id
            && self.mood_group_id == other.mood_group_id
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.name.to_lowercase() == other.name.to_lowercase() && self.icon == other.icon
    }
}

impl Daylio {
    fn change_mood_id(day_entries: &mut [DayEntry], mood: &mut CustomMood, new_id: i64) {
        for entry in day_entries {
            if entry.mood == mood.id {
                entry.mood = new_id;
            }
        }
        mood.id = new_id
    }

    fn change_tag_id(day_entries: &mut [DayEntry], tag: &mut Tag, new_id: i64) {
        for entry in day_entries {
            for i in 0..entry.tags.len() {
                if entry.tags[i] == tag.id {
                    entry.tags[i] = new_id;
                    break;
                }
            }
        }
        tag.id = new_id;
    }

    fn make_ids_distinct(&mut self, gen: &mut IdGenerator) {
        for mood in &mut self.custom_moods {
            Daylio::change_mood_id(&mut self.day_entries, mood, gen.next());
        }

        for tag in &mut self.tags {
            Daylio::change_tag_id(&mut self.day_entries, tag, gen.next());
        }
    }

    fn remove_duplicates(&mut self) {
        // for moods
        self.custom_moods
            .sort_by_key(|x| (x.custom_name.to_lowercase(), x.icon_id));

        for i in 1..self.custom_moods.len() {
            if self.custom_moods[i - 1] == self.custom_moods[i] {
                let new_id = self.custom_moods[i - 1].id;
                Daylio::change_mood_id(&mut self.day_entries, &mut self.custom_moods[i], new_id);
                self.custom_moods[i].id = -1; // mark for deletion
            }
        }

        self.custom_moods.retain(|mood| mood.id != -1);

        // for tags
        self.tags.sort_by_key(|x| (x.name.to_lowercase(), x.icon));

        for i in 1..self.tags.len() {
            if self.tags[i - 1] == self.tags[i] {
                let new_id = self.tags[i - 1].id;
                Daylio::change_tag_id(&mut self.day_entries, &mut self.tags[i], new_id);
                self.tags[i].id = -1; // mark for deletion
            }
        }

        self.tags.retain(|tag| tag.id != -1);

        // for entries
        self.day_entries
            .sort_by_key(|x| (x.datetime, x.year, x.month));

        for i in 1..self.day_entries.len() {
            // we do not want to lose any data, so they need to be exactly the same
            if self.day_entries[i - 1] == self.day_entries[i] {
                self.day_entries[i].id = -1; // mark for deletion
            }
        }

        self.day_entries.retain(|entry| entry.id != -1);
    }

    pub fn sanitize(&mut self) {
        // fix: sometimes custom moods have a custom
        // name and a predefined name
        // we keep custom name and remove predefined name
        for mood in self.custom_moods.iter_mut() {
            if mood.predefined_name_id != -1 && !mood.custom_name.is_empty() {
                mood.predefined_name_id = -1;
            }
        }

        // predefined moods have to have the same id as the predefined name
        for mood in self.custom_moods.iter_mut() {
            if mood.predefined_name_id != -1 {
                Daylio::change_mood_id(&mut self.day_entries, mood, mood.predefined_name_id);
            }
        }

        // we start at 6 because the first 5 predefined moods are reserved
        let mut id_generator = IdGenerator::with_start(1, 6);

        // order is important, so we need to sort by mood_group_id and predefined comes first
        self.custom_moods
            .sort_by_key(|x| (x.mood_group_id, -x.predefined_name_id));
        for mood in self.custom_moods.iter_mut() {
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
        for (i, mut tag) in self.tags.iter_mut().enumerate() {
            Daylio::change_tag_id(&mut self.day_entries, tag, id_generator.next());
            tag.order = i as i64 + 1;
        }

        self.day_entries
            .sort_by_key(|x| (-x.datetime, -x.year, -x.month));
        let mut id_generator = IdGenerator::new(1);
        for entry in self.day_entries.iter_mut() {
            entry.id = id_generator.next();
        }
    }
}

pub fn merge(mut daylio1: Daylio, mut daylio2: Daylio) -> Daylio {
    const BIG_OFFSET: u64 = 1000;

    // first_pass: make sure we don't have any duplicates id
    let mut id_generator = IdGenerator::new(BIG_OFFSET as i64);
    daylio1.make_ids_distinct(&mut id_generator);
    daylio2.make_ids_distinct(&mut id_generator);

    let mut merged = daylio1;
    merged
        .custom_moods
        .append(&mut daylio2.custom_moods.clone());
    merged.tags.append(&mut daylio2.tags.clone());
    merged.day_entries.append(&mut daylio2.day_entries.clone());

    merged.remove_duplicates();
    merged.sanitize();

    // update metadata
    merged.metadata.number_of_entries = merged.day_entries.len() as i64;
    merged.metadata.number_of_photos += daylio2.metadata.number_of_photos;
    merged.metadata.photos_size += daylio2.metadata.photos_size;

    merged
}
