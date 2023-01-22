use crate::daylio::{CustomMood, Daylio, Tag};
use crate::DayEntry;

#[derive(Clone, Copy)]
struct IdGenerator {
    offset: i64,
    current: i64,
}

impl IdGenerator {
    fn new(offset: i64) -> Self {
        Self { offset, current: 0 }
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

pub fn merge(mut daylio1: Daylio, mut daylio2: Daylio) -> Daylio {
    const BIG_OFFSET: u64 = 1000;

    // first_pass: make sure we don't have any duplicates id
    let mut id_generator = IdGenerator::new(BIG_OFFSET as i64);
    for daylio in [&mut daylio1, &mut daylio2].iter_mut() {
        for mood in &mut daylio.custom_moods {
            change_mood_id(&mut daylio.day_entries, mood, id_generator.next());
        }

        for tag in &mut daylio.tags {
            change_tag_id(&mut daylio.day_entries, tag, id_generator.next());
        }
    }

    let mut merged = daylio1.clone();
    merged
        .custom_moods
        .append(&mut daylio2.custom_moods.clone());
    merged.tags.append(&mut daylio2.tags.clone());
    merged.day_entries.append(&mut daylio2.day_entries.clone());

    // second_pass: make sure we don't have any duplicate

    // for moods
    merged
        .custom_moods
        .sort_by_key(|x| (x.custom_name.to_lowercase(), x.icon_id));

    for i in 1..merged.custom_moods.len() {
        if merged.custom_moods[i - 1] == merged.custom_moods[i] {
            let new_id = merged.custom_moods[i - 1].id;
            change_mood_id(&mut merged.day_entries, &mut merged.custom_moods[i], new_id);
            merged.custom_moods[i].id = -1; // mark for deletion
        }
    }

    merged.custom_moods.retain(|mood| mood.id != -1);

    // for tags
    merged.tags.sort_by_key(|x| (x.name.to_lowercase(), x.icon));

    for i in 1..merged.tags.len() {
        if merged.tags[i - 1] == merged.tags[i] {
            let new_id = merged.tags[i - 1].id;
            change_tag_id(&mut merged.day_entries, &mut merged.tags[i], new_id);
            merged.tags[i].id = -1; // mark for deletion
        }
    }

    merged.tags.retain(|tag| tag.id != -1);

    // for entries
    merged
        .day_entries
        .sort_by_key(|x| (x.datetime, x.year, x.month));

    for i in 1..merged.day_entries.len() {
        // we do not want to lose any data, so they need to be exactly the same
        if merged.day_entries[i - 1] == merged.day_entries[i] {
            merged.day_entries[i].id = -1; // mark for deletion
        }
    }

    merged.day_entries.retain(|entry| entry.id != -1);

    // finally, sort by date and update ids
    merged
        .custom_moods
        .sort_by_key(|x| (x.mood_group_id, x.created_at));
    merged.tags.sort_by_key(|x| x.created_at);
    merged
        .day_entries
        .sort_by_key(|x| (-x.datetime, -x.year, -x.month));

    // fix: sometimes custom moods have a custom
    // name and a predefined name
    // we keep custom name and remove predefined name
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id != -1 && !mood.custom_name.is_empty() {
            mood.predefined_name_id = -1;
        }
    }

    // ids start at 1
    let mut id_generator = IdGenerator::new(1);
    // first handle predefined moods
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id != -1 {
            change_mood_id(&mut merged.day_entries, mood, id_generator.next());
        }
    }

    // then handle custom moods
    // order is important, so we need to sort by mood_group_id and predefined comes first
    merged
        .custom_moods
        .sort_by_key(|x| (x.mood_group_id, -x.predefined_name_id));
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id == -1 {
            change_mood_id(&mut merged.day_entries, mood, id_generator.next());
        }
    }

    // each mood_group_id has an order, so we need to update it
    for i in 0..merged.custom_moods.len() {
        if i == 0
            || merged.custom_moods[i].mood_group_id != merged.custom_moods[i - 1].mood_group_id
        {
            merged.custom_moods[i].mood_group_order = 0;
        } else {
            merged.custom_moods[i].mood_group_order =
                merged.custom_moods[i - 1].mood_group_order + 1;
        }
    }

    let mut id_generator = IdGenerator::new(1);
    for (i, mut tag) in merged.tags.iter_mut().enumerate() {
        change_tag_id(&mut merged.day_entries, tag, id_generator.next());
        tag.order = i as i64 + 1;
    }

    let mut id_generator = IdGenerator::new(1);
    for entry in merged.day_entries.iter_mut() {
        entry.id = id_generator.next();
    }

    // update metadata
    merged.metadata.number_of_entries = merged.day_entries.len() as i64;
    merged.metadata.number_of_photos =
        daylio1.metadata.number_of_photos + daylio2.metadata.number_of_photos;
    merged.metadata.photos_size = daylio1.metadata.photos_size + daylio2.metadata.photos_size;

    merged
}
