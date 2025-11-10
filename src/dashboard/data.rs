use serde::{Deserialize, Serialize};

use crate::statistics::DashboardStats;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub version: String,
    pub generated_at: String,
    pub metadata: Metadata,
    pub config: AppliedConfig,
    pub moods: Vec<MoodDetailLite>,
    pub tags: Vec<TagDetailLite>,
    pub entries: Vec<EntryLite>,
    pub stats: DashboardStats,
    pub highlights: Vec<Highlight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub first_date: String,
    pub last_date: String,
    pub total_entries: u32,
    pub total_days_logged: u32,
    pub word_total: u32,
    pub word_median: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedConfig {
    pub period: PeriodSelector,
    pub include_notes: bool,
    pub anonymize_tags: bool,
    pub min_samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PeriodSelector {
    All,
    LastNDays(u32),
    Year(u32),
    YearToDate,
    Range {
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodDetailLite {
    pub name: String,
    pub wellbeing_value: Option<u64>,
    pub category: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDetailLite {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryLite {
    pub dt: String,
    pub moods: Vec<String>,
    pub tags: Vec<String>,
    pub w: u32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub kind: String,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
