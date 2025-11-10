# Dashboard Technical Design

This document maps the functional spec into concrete modules, data contracts, and build steps within this Rust
repository.

High-Level Approach

- Generate a static dashboard from the CLI. No server required.
- Compute analytics in Rust, serialize a versioned DashboardData JSON, and render an HTML template.
- Ship as a static bundle; optionally inline into a single HTML file.

CLI

- New subcommand: `generate-dashboard`
    - Flags (see `cli-and-config.md`): `--input`, `--out-dir`, `--period`, `--include-notes`, `--anonymize-tags`,
      `--single-file`, `--min-samples`

Rust Modules and Responsibilities

- `src/models.rs`
    - Already defines `Diary`, `DayEntry`, `MoodDetail`, `TagDetail`
    - Assumes multiple moods/tags per entry and optional note
- `src/parse_md.rs`
    - Parses Markdown diaries into `Diary`
- `src/statistics.rs` (to implement)
    - Pure functions to compute stats:
        - Per-entry mood score calculation (average of available wellbeing values)
        - Daily aggregates: mood, entries, words
        - Distributions: mood frequency (weighted), tags usage
        - Temporal: weekday/hour aggregates
        - Streaks: logging and writing (thresholded)
        - Correlations: tag impact (present vs absent), min samples guard
        - Calendar readiness: map of date -> daily metrics
    - Expose `compute_dashboard_stats(diary: &Diary, cfg: &StatsConfig) -> DashboardStats`
- `src/dashboard/` (new)
    - `data.rs`: `DashboardData` struct + serde
    - `export.rs`: write bundle files and optional single-file inliner
    - `templates/` (or `templates/` at repo root) containing HTML skeleton

Data Flow

- Load diary from path
- Filter by selected period
- Compute `DashboardStats`
- Assemble `DashboardData { metadata, entries(lightweight), stats, generated_at, version }`
- Write output files

Frontend Choices (static, minimal)

- Charts: Vega-Lite via CDN (or local copy). Data transformations are precomputed in Rust.
- App JS: Vanilla ES modules for reading embedded JSON and configuring charts; minimal state.
- CSS: Simple CSS (e.g., CSS variables + grid); optional Pico.css as a tiny baseline.

Single-File Export

- Create `index.single.html` by inlining:
    - CSS into <style>
    - JS (app + vendor) into <script>
    - Embed JSON payload via `<script id="diary-data" type="application/json">{...}</script>`
- Use base64 for any images (optional)

Versioning

- `DashboardData.version` starts at "1"
- Any breaking change -> bump minor/major and keep a migration note in this doc

Performance & Size

- Expect <20k entries. Keep `data.json` under tens of MB by:
    - Omitting full note text unless `--include-notes`
    - Using concise field names in `DashboardData.entries` if necessary (documented)

Security/Privacy

- No remote requests required to render (avoid CDN in single-file export)
- Redaction options (see privacy doc). Default: omit note text.

Testing Strategy

- Unit tests for stats functions with small fixtures
- Golden tests for `DashboardData` JSON schema (serde roundtrip) with snapshots
- CLI integration test writing a dashboard sample and checking file presence

Error Handling

- Use `color-eyre` across CLI output
- Gracefully degrade when:
    - No moods have numeric values -> skip numeric mood charts and show categorical ones
    - Insufficient samples for correlations -> hide section or show notice

Deliverables (MVP)

- CLI subcommand generating `dashboard/` folder with index.html, data.json, app.js, style.css
- At least 4 charts: KPI summary, mood over time, words per day, tag usage
- Optionally `index.single.html`

# Dashboard Functional Specification

Goal: Provide an at-a-glance and deep-dive view of diary health: mood trends, writing habits, tag themes, and
correlations – entirely offline.

Data Origin

- Diary entries from Markdown (`parse_md.rs`) or Daylio backups converted to `Diary` (`models.rs`)
- Each `DayEntry` has: timestamp (to minute), 0–N moods, 0–N tags, optional note body (string, may be empty)
- Global mood and tag details available via `Diary.moods` / `Diary.tags`

Primary User Questions

1. How have my moods changed recently?
2. What moods/tags are most frequent? Emerging? Fading?
3. Does certain activity (tag) correlate with better/worse mood?
4. How consistently do I write/log?
5. When (time-of-day, weekday) are entries and moods concentrated?
6. What is my writing volume and streak history?
7. What standout patterns should I reflect on?

Functional Sections

1. KPI Summary (top strip)
    - Average mood (current period) vs previous period delta
    - Mood distribution (positive/neutral/negative share or custom buckets)
    - Total entries / days logged / current streak / longest streak
    - Words total and median words per entry
    - Top 3 tags and usage counts
    - Data freshness timestamp
2. Mood Over Time
    - Daily average mood line (optionally smoothed 7-day moving average)
    - Optional stacked area showing mood category composition per day/week
3. Calendar View
    - Heatmap of days (color by mood score; intensity by word count or entry presence)
    - Tooltips: date, moods count, tags count, words
4. Mood Frequency & Distribution
    - Table or bar chart of mood counts (weighted by 1/moods_per_entry)
    - Box or histogram of mood scores (if wellbeing values available)
    - Mood combination frequency (common pairs/sets)
5. Tag Analytics
    - Tag usage count & first/last seen date
    - Impact (difference in average mood when tag present vs absent)
    - Co-occurrence: top tag pairs by joint occurrence
6. Writing Analytics
    - Words per day (bar/line)
    - Entry count per day/week
    - Streaks (current, longest) – logging and writing (>=X words threshold)
7. Temporal Patterns
    - Average mood by weekday
    - Average mood by hour (if distribution meaningful)
    - Entry count by hour & weekday
8. Highlights / Insights (generated text snippets)
    - Period comparison: entries/mood delta
    - Emerging tag (recent spike in usage)
    - Notable correlation (tag strongly associated with above/below baseline mood)
    - Anomaly cluster (several low-mood days in short span)
9. Drill-down Entry Table
    - Filterable list of entries (period, tag inclusion/exclusion, mood set, with/without notes)
    - Columns: date/time, moods, tags, word count, note preview (configurable hide)
10. Privacy Controls (export-time configuration)

- Hide note bodies (only counts)
- Mask tag names (hash or sequential tag_n)
- Omit dates (bucket by week index) for anonymized sharing

Filtering Behavior (Front-end)

- Global date range selector (preset: last 30 days, last 90, YTD, all)
- Tag filter: include any of list, exclude list
- Mood filter: include selected moods
- Toggles: show/hide entries without mood; show/hide note text

Data Quality Rules / Edge Cases

- Multiple moods: per-entry mood score is mean of available wellbeing values; frequency counts weight each mood by
  1/len(moods)
- Missing wellbeing_value: mood used in categorical counts; excluded from numeric averages
- Empty note: word count = 0
- Duplicate tags/moods in entry (should not occur due to `HashSet`) – ignore if encountered
- No mood entries: denominator for average mood excludes those entries

KPIs Precise Definitions

- Average mood: mean of per-entry mood scores within period
- Entries count: number of `DayEntry`
- Days logged: distinct calendar days with >=1 entry
- Streak (logging): max consecutive days with >=1 entry
- Word metrics: total words = sum of `split_whitespace().len()`; median computed across entries with body (>=1 word)
- Tag usage: count of entries containing tag
- Mood distribution: categorize mood score into 5 buckets or custom mapping defined by wellbeing_value quantiles

Output Variants

1. Bundle mode (recommended): /dashboard/index.html + data.json + app.js + style.css + vendor libs (optional local
   copies)
2. Single-file mode: index.single.html with inline CSS & JS & JSON (`<script id="diary-data" type="application/json">`)

Success Criteria

- Opening index.html offline renders all charts and tables
- Export completes under ~2 seconds for <20k entries
- Data contract stable and versioned
- Privacy flags produce redacted dataset

Non-Goals (Initial Phase)

- Authentication / multi-user
- Real-time editing in browser
- Natural language processing beyond simple token frequency

Glossary

- Mood score: numeric wellbeing_value mapped/normalized to [1..N] scale
- Impact: difference (tag present vs absent) in mean mood score
- Emerging tag: usage in last period > 2x previous comparable period (with min count threshold)

See technical design for implementation details.

