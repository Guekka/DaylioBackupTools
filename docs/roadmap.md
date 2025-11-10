# Roadmap

Phase 1 – MVP (Static Bundle)

- Implement `generate-dashboard` CLI
- Compute basic stats: KPI summary, mood daily, words daily, tag usage
- Export `dashboard/` with `index.html`, `data.json`, `app.js`, `style.css`
- Optional: `index.single.html`

Phase 2 – Depth & Patterns

- Add calendar heatmap, weekday/hour patterns
- Add streaks and highlights
- Add tag co-occurrence and combos
- Add anonymization options and config echoes

Phase 3 – Associations & Polishing

- Tag impact and lagged effects with thresholds
- Emerging tags detection
- Improve front-end filters (date, tag include/exclude)
- Accessibility text summaries

Phase 4 – Packaging & DX

- Nicer CLI UX (progress, colored output)
- Local vendor bundling for single-file mode (no CDN)
- Watch mode: regenerate on diary change

Phase 5 – Optional Enhancements

- WASM compute for client-side filters
- Desktop app packaging (Tauri) if requested
- NLP-lite: keyword frequency per period, simple topic groups

Cut Criteria for MVP

- Openable offline
- 4+ charts render correctly
- Data contract v1 stable
- Unit tests cover core functions

