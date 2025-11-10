# Privacy and Anonymization

Default posture: local, offline analytics with no network calls. Provide explicit options to limit sensitive data
exposure in exported artifacts.

Options (CLI flags)

- `--include-notes` (off by default)
    - If off: omit `EntryLite.note` and include only `w` (word count)
- `--anonymize-tags` (off by default)
    - Hash or remap tag names: `tag_1`, `tag_2`, ... Deterministic ordering: sort by original name.
    - Apply to: entries.moods? No; only tags. Moods generally not considered private, but could add a flag later.
- Date bucketing (future)
    - `--bucket-dates=week`: Replace exact dates with `YYYY-Www`

Other Safeguards

- Vendor assets: Prefer local copies in single-file mode to avoid network fetches revealing usage.
- Size considerations: large `note` fields can balloon file sizes. Warn if total exceeds threshold (e.g., 25MB).
- Sensitive-tag exclusion (future): `--exclude-tags SECRET,PRIVATE` to redact entries from outputs or conceal tag
  presence.

Redaction Audit Trail

- Echo applied config into `DashboardData.config`.
- Add a top-of-file HTML comment in single-file export documenting options used and timestamp.

Irreversibility

- Once tags are anonymized and notes removed in exported dashboard, original semantics can't be recovered from artifact
  alone.

