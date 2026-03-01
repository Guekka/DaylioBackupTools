use crate::formats::models::{DayEntry, Diary};
use crate::{Mood, MoodDetail, Tag, TagDetail};
use chrono::TimeDelta;

/// Policy for comparing day entries
/// - Strict: note must be exactly the same
/// - Relaxed: note must be the same after simplification
/// - Contained: one note must be contained in the other after simplification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayEntryComparisonPolicy {
    Strict,
    Relaxed,
    Contained,
}

const MIN_LENGTH_FOR_CONTAINED_COMPARISON: usize = 20;

impl Diary {
    fn find_mood_detail(&self, mood: &Mood) -> Option<&MoodDetail> {
        self.moods.iter().find(|m| m.name == mood.name)
    }

    fn find_tag_detail(&self, tag: &Tag) -> Option<&TagDetail> {
        self.tags.iter().find(|t| t.name == tag.name)
    }
    fn add_entry(&mut self, index: usize, entry: DayEntry, mergee: &Diary) {
        let moods_to_add = entry
            .moods
            .iter()
            .filter(|mood| self.find_mood_detail(mood).is_none())
            .filter_map(|mood| mergee.find_mood_detail(mood))
            .cloned()
            .collect::<Vec<MoodDetail>>();

        let tags_to_add = entry
            .tags
            .iter()
            .filter(|tag| self.find_tag_detail(tag).is_none())
            .filter_map(|tag| mergee.find_tag_detail(tag))
            .cloned()
            .collect::<Vec<TagDetail>>();

        if !moods_to_add.is_empty() {
            println!(
                "Adding {} new mood(s): {:?}",
                moods_to_add.len(),
                moods_to_add
                    .iter()
                    .map(|m| &m.name)
                    .collect::<Vec<&String>>()
            );
        }

        if !tags_to_add.is_empty() {
            println!(
                "Adding {} new tag(s): {:?}",
                tags_to_add.len(),
                tags_to_add
                    .iter()
                    .map(|t| &t.name)
                    .collect::<Vec<&String>>()
            );
        }

        self.moods.extend(moods_to_add);
        self.tags.extend(tags_to_add);

        self.day_entries.insert(index, entry);
    }

    /// Aggressive simplification of the note: only keep alphanumeric characters
    fn simplify_note_for_comparing(entry: &DayEntry) -> String {
        entry
            .note
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    pub fn add_unique_entries(
        &mut self,
        mergee: &mut Diary,
        comparison_policy: DayEntryComparisonPolicy,
    ) {
        let sort_by = |lhs: &DayEntry, rhs: &DayEntry| {
            lhs.date
                .cmp(&rhs.date)
                .then(lhs.moods.iter().cmp(rhs.moods.iter()))
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

            let same_note = match comparison_policy {
                DayEntryComparisonPolicy::Strict => self_entry.note == added_entry.note,
                DayEntryComparisonPolicy::Relaxed | DayEntryComparisonPolicy::Contained => {
                    let self_ = Self::simplify_note_for_comparing(self_entry);
                    let other_ = Self::simplify_note_for_comparing(added_entry);

                    if self_ == other_ {
                        true
                    } else if comparison_policy == DayEntryComparisonPolicy::Contained
                        && self_.len() > MIN_LENGTH_FOR_CONTAINED_COMPARISON
                        && other_.len() > MIN_LENGTH_FOR_CONTAINED_COMPARISON
                        && (self_.contains(&other_) || other_.contains(&self_))
                    {
                        println!(
                            "Contained note detected, keeping the longer one:\n---\n{}\n---\n{}\n---",
                            self_entry.note, added_entry.note
                        );

                        // keep the longer note
                        if self_.len() < other_.len() {
                            self_entry.note.clone_from(&added_entry.note);
                        }

                        true
                    } else {
                        false
                    }
                }
            };

            if same_day && same_note {
                // We keep the one from the reference file
                right_index += 1;
            } else if sort_by(self_entry, added_entry) == std::cmp::Ordering::Less {
                left_index += 1;
            } else {
                let entry = added_entry.clone();
                self.add_entry(left_index, entry, mergee);
                left_index += 1;
                right_index += 1;
            }
        }

        // Add the remaining entries from mergee
        while right_index < mergee.day_entries.len() {
            let added_entry = &mut mergee.day_entries[right_index];
            self.add_entry(self.day_entries.len(), added_entry.clone(), mergee);
            right_index += 1;
        }
    }
}

/// Merges two daylio files into one.
/// We keep everything from the first file, and add the new entries from the other files
pub fn merge(
    mut reference: Diary,
    mut mergee: Diary,
    comparison_policy: DayEntryComparisonPolicy,
) -> color_eyre::Result<Diary> {
    reference.add_unique_entries(&mut mergee, comparison_policy);

    Ok(reference)
}
