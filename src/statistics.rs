//! Statistics and analytics computations for the dashboard.
//! All functions are pure and operate over in-memory data.

use chrono::{Datelike, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{DayEntry, Diary, MoodDetail};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    pub min_samples: usize,
    pub word_threshold: usize,
    pub max_combos: usize,
    pub max_tag_pairs: usize,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            min_samples: 5,
            word_threshold: 10,
            max_combos: 50,
            max_tag_pairs: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardStats {
    pub mood: MoodStats,
    pub tags: TagStats,
    pub writing: WritingStats,
    pub temporal: TemporalStats,
    pub calendar: CalendarStats,
    pub correlations: CorrelationStats,
    pub streaks: StreakStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MoodStats {
    pub daily: Vec<DailyMood>,
    pub distribution: Vec<MoodFrequency>,
    pub combos: Vec<MoodCombo>,
    pub average: Option<f64>,
    pub previous_period_average: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMood {
    pub date: String,
    pub avg: Option<f64>,
    pub entries: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodFrequency {
    pub mood: String,
    pub count: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodCombo {
    pub moods: Vec<String>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagStats {
    pub usage: Vec<TagUsage>,
    pub pairs: Vec<TagPair>,
    pub impact: Vec<TagImpact>,
    pub emerging: Vec<EmergingTag>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUsage {
    pub tag: String,
    pub count: u32,
    pub first: String,
    pub last: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagPair {
    pub tags: [String; 2],
    pub count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagImpact {
    pub tag: String,
    pub delta: f64,
    pub samples: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergingTag {
    pub tag: String,
    pub growth_factor: f64,
    pub previous_count: u32,
    pub current_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WritingStats {
    pub words_daily: Vec<DailyWords>,
    pub entries_daily: Vec<DailyEntries>,
    pub length_hist: Vec<LengthBucket>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyWords {
    pub date: String,
    pub words: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyEntries {
    pub date: String,
    pub entries: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthBucket {
    pub bucket: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalStats {
    pub weekday_mood: Vec<WeekdayMood>,
    pub hour_mood: Vec<HourMood>,
    pub weekday_entries: Vec<WeekdayCount>,
    pub hour_entries: Vec<HourCount>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekdayMood {
    pub weekday: u8,
    pub avg: Option<f64>,
    pub samples: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourMood {
    pub hour: u8,
    pub avg: Option<f64>,
    pub samples: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekdayCount {
    pub weekday: u8,
    pub entries: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourCount {
    pub hour: u8,
    pub entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarStats {
    pub days: Vec<CalendarDay>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDay {
    pub date: String,
    pub mood_avg: Option<f64>,
    pub entries: u32,
    pub words: u32,
    pub moods_count: u32,
    pub tags_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorrelationStats {
    pub tag_impact: Vec<TagImpact>,
    pub lagged: Vec<LaggedTagEffect>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaggedTagEffect {
    pub tag: String,
    pub delta_next_day: f64,
    pub samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreakStats {
    pub logging_current: u32,
    pub logging_longest: u32,
    pub writing_current: u32,
    pub writing_longest: u32,
}

// Helper: per-entry mood score
fn entry_mood_score(entry: &DayEntry, mood_details: &[MoodDetail]) -> Option<f64> {
    if entry.moods.is_empty() {
        return None;
    }
    let mut values = Vec::new();
    for mood in &entry.moods {
        if let Some(detail) = mood_details.iter().find(|m| m.name == mood.name) {
            if let Some(v) = detail.wellbeing_value {
                values.push(v as f64);
            }
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn word_count(entry: &DayEntry) -> usize {
    entry.note.split_whitespace().count()
}

pub fn compute_dashboard_stats(diary: &Diary, cfg: &StatsConfig) -> DashboardStats {
    let mut stats = DashboardStats::default();

    // Group entries by day (YYYY-MM-DD)
    let mut by_day: HashMap<NaiveDate, Vec<&DayEntry>> = HashMap::new();
    for entry in &diary.day_entries {
        by_day.entry(entry.date.date()).or_default().push(entry);
    }

    let mut daily_words = Vec::new();
    let mut daily_entries_vec = Vec::new();
    let mut calendar_days = Vec::new();
    let mut daily_mood_stats = Vec::new();

    let mut all_dates: Vec<NaiveDate> = by_day.keys().cloned().collect();
    all_dates.sort_unstable();

    // Determine full span for calendar (inclusive)
    if let (Some(first), Some(last)) = (all_dates.first().copied(), all_dates.last().copied()) {
        let mut d = first;
        while d <= last {
            let entries = by_day.get(&d).cloned().unwrap_or_default();
            let words: usize = entries.iter().map(|e| word_count(e)).sum();
            let moods_scores: Vec<f64> = entries
                .iter()
                .filter_map(|e| entry_mood_score(e, &diary.moods))
                .collect();
            let day_avg = if moods_scores.is_empty() {
                None
            } else {
                Some(moods_scores.iter().sum::<f64>() / moods_scores.len() as f64)
            };
            let moods_count: u32 = entries.iter().map(|e| e.moods.len() as u32).sum();
            let tags_count: u32 = entries.iter().map(|e| e.tags.len() as u32).sum();
            if !entries.is_empty() {
                daily_words.push(DailyWords {
                    date: d.to_string(),
                    words: words as u32,
                });
                daily_entries_vec.push(DailyEntries {
                    date: d.to_string(),
                    entries: entries.len() as u32,
                });
                daily_mood_stats.push(DailyMood {
                    date: d.to_string(),
                    avg: day_avg,
                    entries: entries.len() as u32,
                });
            }
            calendar_days.push(CalendarDay {
                date: d.to_string(),
                mood_avg: day_avg,
                entries: entries.len() as u32,
                words: words as u32,
                moods_count,
                tags_count,
            });
            d = d.succ_opt().unwrap();
        }
    }

    // Mood distribution and combos
    let mut mood_weight: HashMap<String, f64> = HashMap::new();
    let mut combo_counts: HashMap<Vec<String>, u32> = HashMap::new();
    let mut per_entry_scores = Vec::new();
    for entry in &diary.day_entries {
        if let Some(score) = entry_mood_score(entry, &diary.moods) {
            per_entry_scores.push(score);
        }
        let mcount = entry.moods.len();
        if mcount > 0 {
            let weight = 1.0 / mcount as f64;
            for m in &entry.moods {
                *mood_weight.entry(m.name.clone()).or_insert(0.0) += weight;
            }
        }
        if mcount > 1 {
            let mut names: Vec<String> = entry.moods.iter().map(|m| m.name.clone()).collect();
            names.sort();
            *combo_counts.entry(names).or_insert(0) += 1;
        }
    }

    let mut distribution: Vec<MoodFrequency> = mood_weight
        .into_iter()
        .map(|(mood, count)| MoodFrequency { mood, count })
        .collect();
    distribution.sort_by(|a, b| b.count.partial_cmp(&a.count).unwrap());

    let mut combos: Vec<MoodCombo> = combo_counts
        .into_iter()
        .map(|(moods, count)| MoodCombo { moods, count })
        .collect();
    combos.sort_by(|a, b| b.count.cmp(&a.count));
    if combos.len() > cfg.max_combos {
        combos.truncate(cfg.max_combos);
    }

    let average = if per_entry_scores.is_empty() {
        None
    } else {
        Some(per_entry_scores.iter().sum::<f64>() / per_entry_scores.len() as f64)
    };

    stats.mood = MoodStats {
        daily: daily_mood_stats,
        distribution,
        combos,
        average,
        previous_period_average: None,
    };

    // Tag usage & pairs
    let mut tag_usage: HashMap<String, (u32, NaiveDate, NaiveDate)> = HashMap::new();
    for entry in &diary.day_entries {
        let date = entry.date.date();
        for tag in &entry.tags {
            tag_usage
                .entry(tag.name.clone())
                .and_modify(|(c, f, l)| {
                    *c += 1;
                    if date < *f {
                        *f = date;
                    }
                    if date > *l {
                        *l = date;
                    }
                })
                .or_insert((1, date, date));
        }
    }
    let mut usage_vec: Vec<TagUsage> = tag_usage
        .into_iter()
        .map(|(tag, (count, first, last))| TagUsage {
            tag,
            count,
            first: first.to_string(),
            last: last.to_string(),
        })
        .collect();
    usage_vec.sort_by(|a, b| b.count.cmp(&a.count));

    // Tag pairs
    let mut pair_counts: HashMap<(String, String), u32> = HashMap::new();
    for entry in &diary.day_entries {
        let mut tags: Vec<&String> = entry.tags.iter().map(|t| &t.name).collect();
        tags.sort();
        for i in 0..tags.len() {
            for j in i + 1..tags.len() {
                let a = tags[i].clone();
                let b = tags[j].clone();
                *pair_counts.entry((a.clone(), b.clone())).or_insert(0) += 1;
            }
        }
    }
    let mut pair_vec: Vec<TagPair> = pair_counts
        .into_iter()
        .map(|((a, b), count)| TagPair {
            tags: [a, b],
            count,
        })
        .collect();
    pair_vec.sort_by(|a, b| b.count.cmp(&a.count));
    if pair_vec.len() > cfg.max_tag_pairs {
        pair_vec.truncate(cfg.max_tag_pairs);
    }

    // Emerging tags: split unique days into halves
    let mut emerging_vec: Vec<EmergingTag> = Vec::new();
    let mut unique_days: Vec<NaiveDate> = diary.day_entries.iter().map(|e| e.date.date()).collect();
    unique_days.sort_unstable();
    unique_days.dedup();
    if unique_days.len() >= 2 {
        let mid_idx = unique_days.len() / 2 - 1; // first half inclusive
        let mid_day = unique_days[mid_idx];
        let mut prev_counts: HashMap<String, u32> = HashMap::new();
        let mut curr_counts: HashMap<String, u32> = HashMap::new();
        for entry in &diary.day_entries {
            let d = entry.date.date();
            if d <= mid_day {
                for t in &entry.tags {
                    *prev_counts.entry(t.name.clone()).or_insert(0) += 1;
                }
            } else {
                for t in &entry.tags {
                    *curr_counts.entry(t.name.clone()).or_insert(0) += 1;
                }
            }
        }
        for (tag, curr) in curr_counts.iter() {
            let prev = *prev_counts.get(tag).unwrap_or(&0);
            if *curr >= cfg.min_samples as u32 {
                let growth = (*curr as f64) / (prev.max(1) as f64);
                if growth >= 2.0 {
                    emerging_vec.push(EmergingTag {
                        tag: tag.clone(),
                        growth_factor: growth,
                        previous_count: prev,
                        current_count: *curr,
                    });
                }
            }
        }
        emerging_vec.sort_by(|a, b| {
            b.growth_factor
                .total_cmp(&a.growth_factor)
                .then(b.current_count.cmp(&a.current_count))
        });
    }

    // Tag impact
    let scored_entries: Vec<(&DayEntry, f64)> = diary
        .day_entries
        .iter()
        .filter_map(|e| entry_mood_score(e, &diary.moods).map(|s| (e, s)))
        .collect();
    let global_mean =
        scored_entries.iter().map(|(_, s)| *s).sum::<f64>() / (scored_entries.len().max(1) as f64);
    let mut impact_vec = Vec::new();
    let mut lagged_vec = Vec::new();
    if !scored_entries.is_empty() {
        // index entries by date
        let mut by_date_scored: HashMap<NaiveDate, Vec<f64>> = HashMap::new();
        for (e, s) in &scored_entries {
            by_date_scored.entry(e.date.date()).or_default().push(*s);
        }
        for tag_detail in &diary.tags {
            // iterate over known tags for deterministic order
            let tag = &tag_detail.name;
            let with: Vec<f64> = scored_entries
                .iter()
                .filter(|(e, _)| e.tags.iter().any(|t| &t.name == tag))
                .map(|(_, s)| *s)
                .collect();
            let without: Vec<f64> = scored_entries
                .iter()
                .filter(|(e, _)| !e.tags.iter().any(|t| &t.name == tag))
                .map(|(_, s)| *s)
                .collect();
            if with.len() >= cfg.min_samples && without.len() >= cfg.min_samples {
                let mean_with = with.iter().sum::<f64>() / with.len() as f64;
                let mean_without = without.iter().sum::<f64>() / without.len() as f64;
                impact_vec.push(TagImpact {
                    tag: tag.clone(),
                    delta: mean_with - mean_without,
                    samples: with.len() as u32,
                });
            }
            // Lagged effect: next-day mood after days containing tag vs baseline next-day
            let mut next_day_scores: Vec<f64> = Vec::new();
            for (e, _) in &scored_entries {
                if e.tags.iter().any(|t| &t.name == tag) {
                    let nd = e.date.date().succ_opt().unwrap();
                    if let Some(scores) = by_date_scored.get(&nd) {
                        next_day_scores.extend(scores);
                    }
                }
            }
            if next_day_scores.len() >= cfg.min_samples {
                let mean_next =
                    next_day_scores.iter().copied().sum::<f64>() / next_day_scores.len() as f64;
                lagged_vec.push(LaggedTagEffect {
                    tag: tag.clone(),
                    delta_next_day: mean_next - global_mean,
                    samples: next_day_scores.len() as u32,
                });
            }
        }
    }
    impact_vec.sort_by(|a, b| b.delta.total_cmp(&a.delta));
    lagged_vec.sort_by(|a, b| b.delta_next_day.total_cmp(&a.delta_next_day));

    stats.tags = TagStats {
        usage: usage_vec,
        pairs: pair_vec,
        impact: impact_vec.clone(),
        emerging: emerging_vec,
    };
    stats.correlations = CorrelationStats {
        tag_impact: impact_vec,
        lagged: lagged_vec,
    };

    // Writing length histogram buckets (simple)
    let mut buckets: Vec<(u32, u32)> = vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]; // 0:0-9,1:10-49,2:50-99,3:100-199,4:200+
    for entry in &diary.day_entries {
        let w = word_count(entry) as u32;
        let idx = if w < 10 {
            0
        } else if w < 50 {
            1
        } else if w < 100 {
            2
        } else if w < 200 {
            3
        } else {
            4
        };
        buckets[idx].1 += 1;
    }
    let length_hist = vec![
        LengthBucket {
            bucket: "0-9".into(),
            count: buckets[0].1,
        },
        LengthBucket {
            bucket: "10-49".into(),
            count: buckets[1].1,
        },
        LengthBucket {
            bucket: "50-99".into(),
            count: buckets[2].1,
        },
        LengthBucket {
            bucket: "100-199".into(),
            count: buckets[3].1,
        },
        LengthBucket {
            bucket: "200+".into(),
            count: buckets[4].1,
        },
    ];

    stats.writing = WritingStats {
        words_daily: daily_words,
        entries_daily: daily_entries_vec,
        length_hist,
    };

    // Temporal stats
    let mut weekday_mood: HashMap<u32, (Vec<f64>, u32)> = HashMap::new();
    let mut hour_mood: HashMap<u32, (Vec<f64>, u32)> = HashMap::new();
    let mut weekday_entries: HashMap<u32, u32> = HashMap::new();
    let mut hour_entries: HashMap<u32, u32> = HashMap::new();
    for entry in &diary.day_entries {
        let wd = entry.date.weekday().number_from_monday();
        *weekday_entries.entry(wd).or_insert(0) += 1;
        let hr = entry.date.hour();
        *hour_entries.entry(hr).or_insert(0) += 1;
        if let Some(score) = entry_mood_score(entry, &diary.moods) {
            weekday_mood
                .entry(wd)
                .or_insert((Vec::new(), 0))
                .0
                .push(score);
            weekday_mood.get_mut(&wd).unwrap().1 += 1;
            hour_mood.entry(hr).or_insert((Vec::new(), 0)).0.push(score);
            hour_mood.get_mut(&hr).unwrap().1 += 1;
        }
    }
    let mut weekday_mood_vec: Vec<WeekdayMood> = (1..=7)
        .map(|wd| {
            let (scores, samples) = weekday_mood.remove(&wd).unwrap_or_default();
            let avg = if scores.is_empty() {
                None
            } else {
                Some(scores.iter().sum::<f64>() / scores.len() as f64)
            };
            WeekdayMood {
                weekday: wd as u8,
                avg,
                samples: samples as u32,
            }
        })
        .collect();
    let mut hour_mood_vec: Vec<HourMood> = (0..24)
        .map(|h| {
            let (scores, samples) = hour_mood.remove(&h).unwrap_or_default();
            let avg = if scores.is_empty() {
                None
            } else {
                Some(scores.iter().sum::<f64>() / scores.len() as f64)
            };
            HourMood {
                hour: h as u8,
                avg,
                samples: samples as u32,
            }
        })
        .collect();
    let mut weekday_entries_vec: Vec<WeekdayCount> = (1..=7)
        .map(|wd| WeekdayCount {
            weekday: wd as u8,
            entries: *weekday_entries.get(&wd).unwrap_or(&0) as u32,
        })
        .collect();
    let mut hour_entries_vec: Vec<HourCount> = (0..24)
        .map(|h| HourCount {
            hour: h as u8,
            entries: *hour_entries.get(&h).unwrap_or(&0) as u32,
        })
        .collect();
    weekday_mood_vec.sort_by_key(|w| w.weekday);
    hour_mood_vec.sort_by_key(|h| h.hour);
    weekday_entries_vec.sort_by_key(|w| w.weekday);
    hour_entries_vec.sort_by_key(|h| h.hour);

    stats.temporal = TemporalStats {
        weekday_mood: weekday_mood_vec,
        hour_mood: hour_mood_vec,
        weekday_entries: weekday_entries_vec,
        hour_entries: hour_entries_vec,
    };

    stats.calendar = CalendarStats {
        days: calendar_days,
    };

    // Streaks
    stats.streaks = compute_streaks(&stats.calendar.days, cfg.word_threshold as u32);

    stats
}

fn compute_streaks(days: &[CalendarDay], word_threshold: u32) -> StreakStats {
    let mut logging_current = 0;
    let mut logging_longest = 0;
    let mut writing_current = 0;
    let mut writing_longest = 0;
    let mut any = false;
    for day in days {
        if day.entries > 0 {
            logging_current += 1;
            logging_longest = logging_longest.max(logging_current);
        } else {
            logging_current = 0;
        }
        if day.words >= word_threshold {
            writing_current += 1;
            writing_longest = writing_longest.max(writing_current);
        } else {
            writing_current = 0;
        }
        any = true;
    }
    if !any {
        return StreakStats::default();
    }
    StreakStats {
        logging_current,
        logging_longest,
        writing_current,
        writing_longest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mood, Tag, TagDetail};
    use chrono::NaiveDateTime;

    fn make_entry(date: &str, moods: &[(&str, f64)], tags: &[&str], words: usize) -> DayEntry {
        let dt =
            NaiveDateTime::parse_from_str(&(date.to_string() + " 12:00:00"), "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let note = std::iter::repeat("word")
            .take(words)
            .collect::<Vec<_>>()
            .join(" ");
        let mood_set = moods.iter().map(|(n, _)| Mood::new(n)).collect();
        let tag_set = tags.iter().map(|t| Tag::new(t)).collect();
        DayEntry {
            date: dt,
            moods: mood_set,
            tags: tag_set,
            note,
        }
    }

    #[test]
    fn test_basic_stats() {
        let moods_details = vec![
            MoodDetail {
                name: "Happy".into(),
                icon_id: None,
                wellbeing_value: Some(5),
                category: None,
            },
            MoodDetail {
                name: "Sad".into(),
                icon_id: None,
                wellbeing_value: Some(1),
                category: None,
            },
        ];
        let entries = vec![
            make_entry("2025-01-01", &[("Happy", 5.0)], &["Work"], 8),
            make_entry(
                "2025-01-01",
                &[("Sad", 1.0), ("Happy", 5.0)],
                &["Work", "Play"],
                20,
            ),
            make_entry("2025-01-02", &[("Sad", 1.0)], &["Play"], 55),
        ];
        let diary = Diary {
            day_entries: entries,
            moods: moods_details,
            tags: vec![
                TagDetail {
                    name: "Work".into(),
                    icon_id: None,
                },
                TagDetail {
                    name: "Play".into(),
                    icon_id: None,
                },
            ],
        };
        let stats = compute_dashboard_stats(&diary, &StatsConfig::default());
        assert_eq!(stats.mood.daily.len(), 2);
        assert!(stats.mood.average.is_some());
        assert!(!stats.tags.usage.is_empty());
        assert_eq!(stats.writing.words_daily.len(), 2);
        assert_eq!(stats.streaks.logging_longest, 2);
    }

    #[test]
    fn test_emerging_tags() {
        // Two halves: first half days 01-02, second half days 03-04
        let moods_details = vec![MoodDetail {
            name: "M".into(),
            icon_id: None,
            wellbeing_value: Some(5),
            category: None,
        }];
        let mut entries = Vec::new();
        // First half: tag A appears once, tag B appears twice
        entries.push(make_entry("2025-01-01", &[("M", 5.0)], &["A"], 5));
        entries.push(make_entry("2025-01-02", &[("M", 5.0)], &["B"], 5));
        entries.push(make_entry("2025-01-02", &[("M", 5.0)], &["B"], 5));
        // Second half: tag A appears 3 times (growth 3/1=3), tag B appears 3 times (growth 3/2=1.5)
        entries.push(make_entry("2025-01-03", &[("M", 5.0)], &["A"], 5));
        entries.push(make_entry("2025-01-04", &[("M", 5.0)], &["A"], 5));
        entries.push(make_entry("2025-01-04", &[("M", 5.0)], &["A"], 5));
        entries.push(make_entry("2025-01-03", &[("M", 5.0)], &["B"], 5));
        entries.push(make_entry("2025-01-04", &[("M", 5.0)], &["B"], 5));
        entries.push(make_entry("2025-01-04", &[("M", 5.0)], &["B"], 5));
        let diary = Diary {
            day_entries: entries,
            moods: moods_details,
            tags: vec![
                TagDetail {
                    name: "A".into(),
                    icon_id: None,
                },
                TagDetail {
                    name: "B".into(),
                    icon_id: None,
                },
            ],
        };
        let cfg = StatsConfig {
            min_samples: 2,
            word_threshold: 10,
            max_combos: 50,
            max_tag_pairs: 50,
        };
        let stats = compute_dashboard_stats(&diary, &cfg);
        assert!(stats.tags.emerging.iter().any(|e| e.tag == "A"));
        assert!(!stats.tags.emerging.iter().any(|e| e.tag == "B"));
    }
}
