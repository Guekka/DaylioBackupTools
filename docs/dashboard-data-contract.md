# Dashboard Data Contract (Version 1)

All analytics output consumed by the front-end is defined here. This contract must remain stable for version 1; breaking
changes require version bump and migration notes.

Root Object: `DashboardData`

```text
DashboardData {
  version: String ("1"),
  generated_at: String (ISO8601 UTC timestamp),
  metadata: Metadata,
  config: AppliedConfig,
  moods: [MoodDetailLite],
  tags: [TagDetailLite],
  entries: [EntryLite],
  stats: DashboardStats,
  highlights: [Highlight],
}
```

Metadata

```text
Metadata {
  first_date: String (YYYY-MM-DD),
  last_date: String (YYYY-MM-DD),
  total_entries: u32,
  total_days_logged: u32,
  word_total: u32,
  word_median: Option<u32>,
}
```

AppliedConfig (echo back to front-end for transparency)

```text
AppliedConfig {
  period: PeriodSelector,
  include_notes: bool,
  anonymize_tags: bool,
  min_samples: u32,
}

PeriodSelector = { kind: String, from: String, to: String }
```

MoodDetailLite

```text
MoodDetailLite {
  name: String,
  wellbeing_value: Option<u64>, // Raw numeric from source
  category: Option<String>, // Optional mapping (e.g., Positive)
}
```

TagDetailLite

```text
TagDetailLite { name: String }
```

EntryLite

```text
EntryLite {
  dt: String (ISO8601 e.g. 2025-10-04T14:15:00Z),
  moods: [String],
  tags: [String],
  w: u32,          // word count
  note: Option<String>, // present only if include_notes=true
}
```

DashboardStats

```text
DashboardStats {
  mood: MoodStats,
  tags: TagStats,
  writing: WritingStats,
  temporal: TemporalStats,
  calendar: CalendarStats,
  correlations: CorrelationStats,
  streaks: StreakStats,
}
```

MoodStats

```text
MoodStats {
  daily: [DailyMood],
  distribution: [MoodFrequency],
  combos: [MoodCombo],
  average: Option<f64>,
  previous_period_average: Option<f64>,
}

DailyMood { date: String (YYYY-MM-DD), avg: Option<f64>, entries: u32 }
MoodFrequency { mood: String, count: f64 } // weighted by 1/moods_per_entry
MoodCombo { moods: [String], count: u32 }
```

TagStats

```text
TagStats {
  usage: [TagUsage],
  pairs: [TagPair],
  impact: [TagImpact],
  emerging: [EmergingTag],
}

TagUsage { tag: String, count: u32, first: String, last: String }
TagPair { tags: [String; 2], count: u32 }
TagImpact { tag: String, delta: f64, samples: u32 }
EmergingTag { tag: String, growth_factor: f64, previous_count: u32, current_count: u32 }
```

WritingStats

```text
WritingStats {
  words_daily: [DailyWords],
  entries_daily: [DailyEntries],
  length_hist: [LengthBucket],
}

DailyWords { date: String, words: u32 }
DailyEntries { date: String, entries: u32 }
LengthBucket { bucket: String, count: u32 } // e.g., "0-9", "10-49", etc.
```

TemporalStats

```text
TemporalStats {
  weekday_mood: [WeekdayMood],
  hour_mood: [HourMood],
  weekday_entries: [WeekdayCount],
  hour_entries: [HourCount],
}

WeekdayMood { weekday: u8 (1=Mon..7=Sun), avg: Option<f64>, samples: u32 }
HourMood { hour: u8 (0..23), avg: Option<f64>, samples: u32 }
WeekdayCount { weekday: u8, entries: u32 }
HourCount { hour: u8, entries: u32 }
```

CalendarStats

```text
CalendarStats { days: [CalendarDay] }
CalendarDay {
  date: String (YYYY-MM-DD),
  mood_avg: Option<f64>,
  entries: u32,
  words: u32,
  moods_count: u32,
  tags_count: u32,
}
```

CorrelationStats

```text
CorrelationStats {
  tag_impact: [TagImpact], // duplicate of TagStats.impact for convenience
  lagged: [LaggedTagEffect],
}
LaggedTagEffect { tag: String, delta_next_day: f64, samples: u32 }
```

StreakStats

```text
StreakStats {
  logging_current: u32,
  logging_longest: u32,
  writing_current: u32, // word threshold defined in config (e.g., >=10 words)
  writing_longest: u32,
}
```

Highlights

```text
Highlight { kind: String, message: String, data: Option<serde_json::Value> }
```

Field Semantics & Rules

- All averages exclude entries without a numeric mood score.
- Mood score computation: average of wellbeing_value of all moods in entry. If any mood lacks numeric value, those moods
  are omitted; if none have numeric, entry excluded.
- Tag impact delta: (mean mood when tag present) - (mean mood when absent). Only included if samples >= min_samples.
- Emerging tag: current_period_count >= 2 * previous_period_count AND current_period_count >= min_samples.

Serialization Options

- Use snake_case in Rust structs; JSON output uses same for simplicity.
- Floating numbers: round to 4 decimal places where convenient for size (front-end may reformat).

Extensibility Notes

- Add `ext` field at root later for experimental metrics.
- For v2, consider adding sentiment metrics or clustering outputs.

