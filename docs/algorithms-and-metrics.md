# Algorithms and Metrics

This document defines the precise calculations used for the dashboard.

Conventions

- Dates are grouped by local calendar day (derive from `NaiveDateTime` as provided).
- Word count: `note.split_whitespace().count()`; includes numbers and symbols delimited by whitespace.
- For averages, use `f64` and round at presentation time.

Per-Entry Mood Score

- Input: `DayEntry.moods: HashSet<Mood>` and `Diary.moods: [MoodDetail]`
- For each mood in entry, look up `wellbeing_value` (if present). Compute mean over all moods that have a numeric value.
- If no moods present or none with numeric value -> `None` mood score for that entry.

Daily Aggregation

- Group entries by `YYYY-MM-DD`.
- Daily average mood: mean of per-entry mood scores (exclude `None`).
- Daily words: sum of word counts.
- Daily entries: count of entries.
- Daily moods_count: sum of number of moods per entry.
- Daily tags_count: sum of number of tags per entry.

Mood Distribution (Frequency)

- Count occurrences weighted by `1 / (#moods in that entry)` for each mood present.
- Produce `MoodFrequency { mood, count }` with fractional counts allowed.

Mood Combos

- For entries with 2+ moods, sort mood names lexicographically, use as set key.
- Count frequency across entries. Output top N (`max_combos`).

Tag Usage

- For each tag, count number of entries containing it.
- Track first and last date seen for that tag.

Tag Co-occurrence

- For each entry, consider all unordered pairs of tags (unique). Count frequency.
- Output top N (`max_tag_pairs`).

Tag Impact (Association with Mood)

- Let S_all be set of entries with a numeric mood score.
- For a given tag T:
    - S_T = entries in S_all containing T
    - S_not_T = entries in S_all not containing T
    - If `|S_T| < min_samples` or `|S_not_T| < min_samples`, skip.
    - delta = mean(S_T.mood) - mean(S_not_T.mood)
- Output `TagImpact { tag, delta, samples: |S_T| }`.
- Note: This is an association, not causation. No statistical test in MVP.

Lagged Tag Effect (Next-Day)

- For tag T:
    - For each entry e with date d in S_T, find next-day entries (date == d+1 day) and collect their mood scores.
    - Compare mean(next-day after T) vs baseline mean of all next-day entries across the period.
    - Samples must meet `min_samples`.

Temporal Patterns

- Weekday (1=Mon..7=Sun): average mood and entry counts.
- Hour (0..23): average mood and entry counts.

Streaks

- Logging streak: consecutive days with >=1 entry.
- Writing streak: consecutive days with words >= `word_threshold`.
- Current streak: counts up to the last date in period if that day qualifies.
- Longest streak: max over entire period.

Calendar Data

- For each day in the period (including days without entries), compute
    - entries (0 if none)
    - words (0 if none)
    - mood_avg (None if no numeric moods)
    - moods_count, tags_count

Emerging Tags

- Split period into two halves (or previous comparable period, if specified by CLI).
- growth_factor = current_count / max(1, previous_count)
- Return tags with growth_factor >= 2 and current_count >= min_samples.

Precision and Stability

- Round in front-end only. Keep full-precision `f64` in JSON to avoid compounded rounding.

Insufficient Data Handling

- No numeric moods: return `average=None` and skip charts that require numbers; categorical stats still computed.
- For empty result sets (e.g., no tag pairs), include empty arrays to keep schema stable.

