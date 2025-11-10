# Testing and Quality

Objective: Ensure the dashboard generator is correct, stable, and maintainable.

Quality Gates (run before release)

1) Build: `cargo build`
2) Unit tests: `cargo test`
3) Lint/Typecheck: (optional) add `cargo clippy` and `cargo fmt -- --check`
4) Smoke test: generate dashboard from a small fixture and open in a browser

Unit Tests (Rust)

- Stats functions in `statistics.rs`:
    - Mood score calculation with multiple moods (weighted mean)
    - Daily aggregation (words/entries)
    - Streak calculation (current and longest)
    - Tag impact with insufficient samples
- Data contract serialization
    - Create a small `Diary` fixture; compute stats; serialize `DashboardData`
    - Assert presence of fields and basic invariants (e.g., dates sorted, non-negative counts)

Integration Tests

- CLI `generate-dashboard`:
    - Generate to a temp dir
    - Assert files exist: `index.html`, `data.json`, `app.js`, `style.css`
    - Optionally parse `data.json` and verify version

Golden Files (Optional)

- Keep a sample `data.json` under `tests/data/` and compare with computed output (allowing for timestamps to differ).

Manual QA Checklist

- Open dashboard and verify charts with both numeric and non-numeric mood datasets
- Try with anonymization on/off
- Try large note bodies excluded vs included
- Verify calendar shows empty days

Performance Checks

- Use `cargo bench` (optional) for large synthetic datasets
- Ensure generation under ~2 seconds for up to 20k entries on a typical laptop

Error Messages

- Friendly, actionable messages for empty datasets, invalid period specs, and IO errors

