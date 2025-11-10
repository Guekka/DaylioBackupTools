# Daylio Tools – Dashboard Specs

Purpose: Define a practical, offline-friendly dashboard for the Markdown-and-Daylio-based diary, plus a concrete
technical plan to implement it in this repository.

Audience: Contributors implementing the dashboard using this codebase. Assumption: You only have this repo and the docs
here.

What you’ll build

- A static dashboard (no server) that visualizes moods, tags, writing volume, and patterns over time.
- Output as a static bundle (recommended) with an optional single-file export.
- Rust does all data loading and analytics; the front-end renders charts and interactions.

Start here

- Read Functional Spec: `dashboard-functional-spec.md`
- Then Technical Design: `dashboard-technical-design.md`
- Data contract to implement: `dashboard-data-contract.md`
- Algorithms and metrics definitions: `algorithms-and-metrics.md`
- CLI and configuration: `cli-and-config.md`
- Front-end/templates guidance: `templates-and-frontend.md`
- Privacy/Anonymization options: `privacy-and-anonymization.md`
- Testing and quality gates: `testing-and-quality.md`
- Roadmap and phases: `roadmap.md`

Repository context (key files)

- `src/models.rs` – Diary model used across the tool
- `src/parse_md.rs` – Markdown diary parser (multiple moods, tags, optional note)
- `src/daylio.rs` – Daylio import/export model
- `src/main.rs` – CLI entry points (merge/extract/pack)
- `src/statistics.rs` – Currently empty; planned home for analytics functions

Implementation order (suggested)

1) Finalize the data contract and algorithms
2) Add CLI subcommand to generate dashboard output
3) Implement analytics in `statistics.rs`
4) Implement exporter (serialize JSON + render template)
5) Add minimal front-end and charts
6) Add single-file export and privacy options
7) Write tests, docs updates, and examples

If you have to choose, prefer correctness and clear data contracts over flashy interactivity. The dashboard must be
reproducible and offline-capable.

