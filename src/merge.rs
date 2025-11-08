use crate::models::{DayEntry, Diary};
use chrono::TimeDelta;

impl Diary {
    /// Aggressive simplification of the note: only keep alphanumeric characters
    fn simplify_note_for_comparing(entry: &DayEntry) -> String {
        entry
            .note
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    pub fn add_unique_entries(&mut self, mergee: &mut Diary) {
        let sort_by = |lhs: &DayEntry, rhs: &DayEntry| {
            lhs.date
                .cmp(&rhs.date)
                .then(lhs.mood.cmp(&rhs.mood))
                .then(lhs.tags.iter().cmp(rhs.tags.iter()))
        };

        self.day_entries.sort_by(sort_by);
        mergee.day_entries.sort_by(sort_by);

        let mut left_index = 0;
        let mut right_index = 0;

        while left_index < self.day_entries.len() && right_index < mergee.day_entries.len() {
            let self_entry = &mut self.day_entries[left_index];
            let added_entry = &mut mergee.day_entries[right_index];

            let timestamp_diff = (self_entry.date - added_entry.date).abs();
            let same_day = timestamp_diff < TimeDelta::days(1);

            let same_note = Self::simplify_note_for_comparing(self_entry)
                == Self::simplify_note_for_comparing(added_entry);

            if same_day && same_note {
                // We keep the one from the reference file
                right_index += 1;
            } else if sort_by(self_entry, added_entry) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                let entry = added_entry.clone();
                self.day_entries.insert(left_index, entry.clone());
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining entries from mergee
        while right_index < mergee.day_entries.len() {
            let added_entry = &mut mergee.day_entries[right_index];
            let entry = added_entry.clone();
            self.day_entries.insert(left_index, entry.clone());
            right_index += 1;
        }
    }
}

/// Merges two daylio files into one.
/// We keep everything from the first file, and add the new entries from the other files
pub fn merge(mut reference: Diary, mut mergee: Diary) -> color_eyre::Result<Diary> {
    reference.add_unique_entries(&mut mergee);

    Ok(reference)
}
