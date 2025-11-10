# Implementation Checklist (MVP to Phase 2)

This is a practical, step-by-step list to implement the dashboard using only this repo.

Prep

- [ ] Skim `docs/README.md` and the specs referenced there
- [ ] Review `src/models.rs` and `src/parse_md.rs` to understand `Diary`

1) Data Structures

- [ ] Create new module `src/dashboard/data.rs` with structs defined in `docs/dashboard-data-contract.md` (v1)
- [ ] Add `mod dashboard;` in `src/lib.rs` and re-export `DashboardData`
- [ ] Write serde unit tests to ensure roundtrip JSON

2) Statistics Engine

- [ ] Implement `src/statistics.rs` with pure functions per `docs/algorithms-and-metrics.md`
    - [ ] Per-entry mood score
    - [ ] Daily aggregates (mood/entries/words)
    - [ ] Mood frequency (weighted), combos (limit N)
    - [ ] Tag usage (first/last date)
    - [ ] Tag pairs (limit N)
    - [ ] Temporal (weekday/hour averages, counts)
    - [ ] Streaks (logging, writing threshold)
    - [ ] Tag impact (min_samples threshold)
    - [ ] Calendar map for all days in period
- [ ] Add `compute_dashboard_stats(&Diary, &StatsConfig) -> DashboardStats`
- [ ] Unit tests for edge cases (no moods, multi-moods, sparsity)

3) Exporter

- [ ] Create `src/dashboard/export.rs` exposing:
    - [ ] `write_bundle(output_dir, data: &DashboardData) -> Result<()>`
    - [ ] `write_single_file(output_file, data: &DashboardData) -> Result<()>`
- [ ] Ship template under `templates/index.html` (Askama or Tera)
- [ ] Bundle writes:
    - [ ] `index.html` (template referencing `app.js`, `style.css`, and local `vendor/*.js`)
    - [ ] `data.json` (serialized `DashboardData`)
    - [ ] `app.js` (minimal, see `docs/templates-and-frontend.md`)
    - [ ] `style.css`
    - [ ] `vendor/vega*.min.js` (checked-in or copied from a known location)
- [ ] Single-file writer: inline CSS/JS/JSON and emit `index.single.html`

4) CLI

- [ ] In `src/main.rs`, add subcommand `generate-dashboard` with flags per `docs/cli-and-config.md`
- [ ] Input detection: Markdown vs Daylio backup
- [ ] Apply period filter
- [ ] Apply anonymization
- [ ] Construct `DashboardData` and write bundle (and optional single-file)
- [ ] Print summary to stdout

5) Front-end (minimal)

- [ ] `app.js` to render:
    - [ ] KPI summary
    - [ ] Mood over time (line via Vega-Lite)
    - [ ] Words per day (bar/line)
    - [ ] Tag usage (table or bar)
- [ ] `style.css` basic layout and typography
- [ ] Manual smoke-test: open `index.html` in browser

6) Tests & QA

- [ ] Add unit tests for statistics and JSON contract
- [ ] Add integration test for CLI (writes files into temp dir)
- [ ] Update README with quickstart commands

7) Phase 2 Enhancements

- [ ] Calendar heatmap
- [ ] Temporal patterns (weekday/hour)
- [ ] Streaks section in UI
- [ ] Highlights generation
- [ ] Co-occurrence and impact visualizations

Notes

- Respect privacy defaults: do NOT include notes unless `--include-notes` provided.
- Keep `DashboardData.version = "1"` and bump if schema changes.

