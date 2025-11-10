# CLI and Configuration

New Subcommand: `generate-dashboard`

Basic Invocation

```bash
cargo run -- generate-dashboard --input diary.md --out-dir dashboard/
```

Required Flags

- `--input <PATH>`: Path to source diary (Markdown or Daylio backup). Accept multiple? For MVP single input.
- `--out-dir <PATH>`: Output directory for dashboard bundle.

Optional Flags

- `--period <SPEC>`: Time range selector. Supported values:
    - `all` (default)
    - `last30`
    - `last90`
    - `ytd`
    - `year:2025` (specific year)
    - `from:YYYY-MM-DD,to:YYYY-MM-DD` (explicit)
- `--include-notes`: Include full note text in `entries.note`.
- `--anonymize-tags`: Replace tag names with `tag_1`, `tag_2`, preserving deterministic mapping.
- `--single-file`: Produce a standalone `index.single.html` (inline assets & data).
- `--min-samples <N>`: Minimum sample size for correlation/impact (default: 5).
- `--word-threshold <N>`: Minimum words to count an entry toward writing streak (default: 10).
- `--max-combos <N>`: Limit number of mood combos retained (default: 50).
- `--max-tag-pairs <N>`: Limit tag pair outputs (default: 50).

Config Structs

```rust
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

pub enum PeriodSelector {
    All,
    LastNDays(u32),
    Year(u32),
    YearToDate,
    Range { from: NaiveDate, to: NaiveDate },
}
```

Period Application

- Convert entries to filtered vector by inclusive date range.
- If filtering results in empty dataset, abort with friendly error.

Anonymization

- Build mapping: sorted unique tag names -> sequential ids.
- Replace in entries, tag lists, stats outputs.

Single-File Generation

- After bundle write, create `index.single.html`.
- Include hash comment at top: `<!-- generated: TIMESTAMP sha256:XYZ -->`.

Exit Codes

- 0 success
- 2 invalid period specification
- 3 input not found
- 4 empty result after filtering

Logging (stdout)

- Summaries: number of entries loaded, period applied, output path.
- Warnings: insufficient samples for correlations; missing numeric mood values.

Examples

```bash
# Generate all-time dashboard (bundle)
cargo run -- generate-dashboard --input diary.md --out-dir dashboard/

# Last 30 days, anonymize tags, include notes
cargo run -- generate-dashboard --input diary.md --out-dir dash_last30 --period last30 --include-notes --anonymize-tags

# Specific date range single file
cargo run -- generate-dashboard --input diary.md --out-dir dash_range --period from:2025-01-01,to:2025-06-30 --single-file
```

Future Config Extensions (not MVP)

- `--exclude-tags <TAG,...>`
- `--only-tags <TAG,...>`
- `--exclude-moods <MOOD,...>`
- `--json-out <PATH>` for raw data only

