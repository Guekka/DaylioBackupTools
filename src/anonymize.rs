use crate::Daylio;
use nanorand::{Rng, WyRand};

fn rand_string(len: usize) -> String {
    let mut rng = WyRand::new();
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        s.push(rng.generate_range(65u8..90) as char);
    }
    s
}

pub fn anonymize(daylio: &mut Daylio) {
    for (i, mood) in daylio.custom_moods.iter_mut().enumerate() {
        mood.custom_name = format!("Mood {} {}", i, rand_string(3));
    }

    for (i, tag) in daylio.tags.iter_mut().enumerate() {
        tag.name = format!("Tag {} {}", i, rand_string(3));
    }

    for (i, entry) in daylio.day_entries.iter_mut().enumerate() {
        entry.note = format!("Note {} {}", i, rand_string(3));
        entry.time_zone_offset = 0;
        entry.note_title = format!("Note title {} {}", i, rand_string(3));
    }

    for (i, group) in daylio.tag_groups.iter_mut().enumerate() {
        group.name = format!("Group {} {}", i, rand_string(3));
    }

    for (i, template) in daylio.writing_templates.iter_mut().enumerate() {
        template.body = format!("Template {} {}", i, rand_string(3));
        template.title = format!("Template title {} {}", i, rand_string(3));
    }
}
