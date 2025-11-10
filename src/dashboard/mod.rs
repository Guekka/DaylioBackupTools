// filepath: /home/edgar/code/daylio_tools/src/dashboard/mod.rs
pub mod data;
pub mod export;

use chrono::{Datelike, NaiveDate, Utc};
use std::collections::{HashMap, HashSet};

use crate::statistics::{StatsConfig, compute_dashboard_stats};
use crate::{DayEntry, Diary, MoodDetail, Tag, TagDetail};
use data::{
    AppliedConfig, DashboardData, EntryLite, Metadata, MoodDetailLite, PeriodSelector,
    TagDetailLite,
};

#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub period: PeriodSelector,
    pub include_notes: bool,
    pub anonymize_tags: bool,
    pub single_file: bool,
    pub min_samples: usize,
    pub word_threshold: usize,
    pub max_combos: usize,
    pub max_tag_pairs: usize,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            period: PeriodSelector::All,
            include_notes: false,
            anonymize_tags: false,
            single_file: false,
            min_samples: 5,
            word_threshold: 10,
            max_combos: 50,
            max_tag_pairs: 50,
        }
    }
}

pub fn apply_period(diary: &Diary, period: &PeriodSelector) -> Diary {
    match period {
        PeriodSelector::All => diary.clone(),
        PeriodSelector::LastNDays(n) => {
            // find last date
            let last = diary.day_entries.iter().map(|e| e.date.date()).max();
            if let Some(last) = last {
                let from = last - chrono::Days::new(*n as u64);
                filter_range(diary, from, last)
            } else {
                diary.clone()
            }
        }
        PeriodSelector::Year(y) => {
            let from = NaiveDate::from_ymd_opt(*y as i32, 1, 1).unwrap();
            let to = NaiveDate::from_ymd_opt(*y as i32, 12, 31).unwrap();
            filter_range(diary, from, to)
        }
        PeriodSelector::YearToDate => {
            // Determine last date in diary for context of YTD
            if let Some(last) = diary.day_entries.iter().map(|e| e.date.date()).max() {
                let from = NaiveDate::from_ymd_opt(last.year(), 1, 1).unwrap();
                filter_range(diary, from, last)
            } else {
                diary.clone()
            }
        }
        PeriodSelector::Range { from, to } => filter_range(diary, *from, *to),
    }
}

fn filter_range(diary: &Diary, from: NaiveDate, to: NaiveDate) -> Diary {
    let entries: Vec<DayEntry> = diary
        .day_entries
        .iter()
        .filter(|e| {
            let d = e.date.date();
            d >= from && d <= to
        })
        .cloned()
        .collect();
    // Keep only moods/tags that are used in the filtered entries
    let used_moods: HashSet<String> = entries
        .iter()
        .flat_map(|e| e.moods.iter().map(|m| m.name.clone()))
        .collect();
    let used_tags: HashSet<String> = entries
        .iter()
        .flat_map(|e| e.tags.iter().map(|t| t.name.clone()))
        .collect();
    let moods: Vec<MoodDetail> = diary
        .moods
        .iter()
        .filter(|m| used_moods.contains(&m.name))
        .cloned()
        .collect();
    let tags: Vec<TagDetail> = diary
        .tags
        .iter()
        .filter(|t| used_tags.contains(&t.name))
        .cloned()
        .collect();
    Diary {
        day_entries: entries,
        moods,
        tags,
    }
}

fn anonymize_tags_if_needed(mut diary: Diary, anonymize: bool) -> (Diary, HashMap<String, String>) {
    if !anonymize {
        return (diary, HashMap::new());
    }
    // Deterministic mapping: sorted unique tag names -> tag_1, tag_2, ...
    let mut names: Vec<String> = diary.tags.iter().map(|t| t.name.clone()).collect();
    names.sort();
    names.dedup();
    let mapping: HashMap<String, String> = names
        .into_iter()
        .enumerate()
        .map(|(i, name)| (name, format!("tag_{}", i + 1)))
        .collect();
    for entry in &mut diary.day_entries {
        entry.tags = entry
            .tags
            .iter()
            .map(|t| Tag::new(mapping.get(&t.name).unwrap()))
            .collect();
    }
    for t in &mut diary.tags {
        if let Some(new) = mapping.get(&t.name) {
            t.name = new.clone();
        }
    }
    (diary, mapping)
}

pub fn generate_dashboard_data(diary: &Diary, cfg: &DashboardConfig) -> DashboardData {
    let filtered = apply_period(diary, &cfg.period);
    let (filtered, _mapping) = anonymize_tags_if_needed(filtered, cfg.anonymize_tags);

    // Build entries lite
    let mut entries_lite = Vec::new();
    for e in &filtered.day_entries {
        let dt_iso = {
            let s = e.date.and_utc().to_rfc3339();
            if s.ends_with("+00:00") {
                format!("{}Z", &s[..s.len() - 6])
            } else {
                s
            }
        };
        let w = e.note.split_whitespace().count() as u32;
        entries_lite.push(EntryLite {
            dt: dt_iso,
            moods: e.moods.iter().map(|m| m.name.clone()).collect(),
            tags: e.tags.iter().map(|t| t.name.clone()).collect(),
            w,
            note: if cfg.include_notes {
                Some(e.note.clone())
            } else {
                None
            },
        });
    }

    // Moods and tags lite
    let moods_lite: Vec<MoodDetailLite> = filtered
        .moods
        .iter()
        .map(|m| MoodDetailLite {
            name: m.name.clone(),
            wellbeing_value: m.wellbeing_value,
            category: m.category.clone(),
        })
        .collect();
    let tags_lite: Vec<TagDetailLite> = filtered
        .tags
        .iter()
        .map(|t| TagDetailLite {
            name: t.name.clone(),
        })
        .collect();

    // Stats
    let stats_cfg = StatsConfig {
        min_samples: cfg.min_samples,
        word_threshold: cfg.word_threshold,
        max_combos: cfg.max_combos,
        max_tag_pairs: cfg.max_tag_pairs,
    };
    let mut stats = compute_dashboard_stats(&filtered, &stats_cfg);

    // previous period average
    stats.mood.previous_period_average = previous_period_average(diary, &cfg.period, &stats_cfg);

    // Metadata
    let first_date = filtered.day_entries.iter().map(|e| e.date.date()).min();
    let last_date = filtered.day_entries.iter().map(|e| e.date.date()).max();
    let total_entries = filtered.day_entries.len() as u32;
    let mut days: HashSet<String> = HashSet::new();
    for e in &filtered.day_entries {
        days.insert(e.date.date().to_string());
    }
    let total_days_logged = days.len() as u32;
    let word_total: u32 = filtered
        .day_entries
        .iter()
        .map(|e| e.note.split_whitespace().count() as u32)
        .sum();
    let mut nonzero_words: Vec<u32> = filtered
        .day_entries
        .iter()
        .map(|e| e.note.split_whitespace().count() as u32)
        .filter(|w| *w > 0)
        .collect();
    nonzero_words.sort_unstable();
    let word_median = if nonzero_words.is_empty() {
        None
    } else {
        Some(nonzero_words[nonzero_words.len() / 2])
    };

    let metadata = Metadata {
        first_date: first_date.map(|d| d.to_string()).unwrap_or_default(),
        last_date: last_date.map(|d| d.to_string()).unwrap_or_default(),
        total_entries,
        total_days_logged,
        word_total,
        word_median,
    };

    let generated_at = Utc::now().to_rfc3339();
    let config_applied = AppliedConfig {
        period: cfg.period.clone(),
        include_notes: cfg.include_notes,
        anonymize_tags: cfg.anonymize_tags,
        min_samples: cfg.min_samples as u32,
    };

    DashboardData {
        version: "1".into(),
        generated_at,
        metadata,
        config: config_applied,
        moods: moods_lite,
        tags: tags_lite,
        entries: entries_lite,
        stats,
        highlights: Vec::new(),
    }
}

fn previous_period_average(
    diary: &Diary,
    period: &PeriodSelector,
    cfg: &StatsConfig,
) -> Option<f64> {
    // Compute previous period range matching the selected period; for All, return None
    let range = match period {
        PeriodSelector::All => return None,
        PeriodSelector::LastNDays(n) => {
            let last = diary.day_entries.iter().map(|e| e.date.date()).max()?;
            let from = last - chrono::Days::new(*n as u64);
            let prev_to = from - chrono::Days::new(1);
            let prev_from = prev_to - chrono::Days::new(*n as u64);
            (prev_from, prev_to)
        }
        PeriodSelector::Year(y) => {
            let from = NaiveDate::from_ymd_opt(*y as i32, 1, 1)?;
            let _to = NaiveDate::from_ymd_opt(*y as i32, 12, 31)?; // underscore to silence warning
            let prev_to = from - chrono::Days::new(1);
            let prev_from = NaiveDate::from_ymd_opt(prev_to.year(), 1, 1)?;
            (prev_from, prev_to)
        }
        PeriodSelector::YearToDate => {
            let last = diary.day_entries.iter().map(|e| e.date.date()).max()?;
            let from = NaiveDate::from_ymd_opt(last.year(), 1, 1)?;
            let span = (last - from).num_days() as u64 + 1;
            let prev_to = from - chrono::Days::new(1);
            let prev_from = prev_to - chrono::Days::new(span);
            (prev_from, prev_to)
        }
        PeriodSelector::Range { from, to } => {
            let span = (*to - *from).num_days() as u64 + 1;
            let prev_to = *from - chrono::Days::new(1);
            let prev_from = prev_to - chrono::Days::new(span);
            (prev_from, prev_to)
        }
    };
    let prev = crate::dashboard::filter_range(diary, range.0, range.1);
    let stats = compute_dashboard_stats(&prev, cfg);
    stats.mood.average
}
