use crate::Daylio;

pub fn anonymize(daylio: &mut Daylio) {
    for (i, mood) in daylio.custom_moods.iter_mut().enumerate() {
        mood.custom_name = format!("Mood {}", i);
    }

    for (i, tag) in daylio.tags.iter_mut().enumerate() {
        tag.name = format!("Tag {}", i);
    }

    for entry in daylio.day_entries.iter_mut() {
        entry.note = format!("Note {}", entry.id);
        entry.time_zone_offset = 0;
        entry.note_title = format!("Note title {}", entry.id);
    }

    for group in daylio.tag_groups.iter_mut() {
        group.name = format!("Group {}", group.id);
    }

    for template in daylio.writing_templates.iter_mut() {
        template.body = format!("Template {}", template.id);
        template.title = format!("Template title {}", template.id);
    }
}
