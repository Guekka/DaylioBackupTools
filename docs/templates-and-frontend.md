# Templates and Front-end Guidance

Objective: Minimal JavaScript and HTML/CSS to render the dashboard. All heavy computation done in Rust.

Template Strategy

- Use a single HTML template with placeholders for sections; embed JSON via
  `<script id="diary-data" type="application/json">` block.
- Template engine options:
    - Askama (compile-time checks) – recommended
    - Tera (runtime flexibility)
- Put templates under `templates/` or `src/dashboard/templates/`.

Example Skeleton (bundle mode – local vendor copies)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8"/>
    <title>Diary Dashboard</title>
    <meta name="viewport" content="width=device-width,initial-scale=1"/>
    <link rel="stylesheet" href="style.css"/>
    <!-- Local vendor JS (place minified files under vendor/) -->
    <script src="vendor/vega.min.js"></script>
    <script src="vendor/vega-lite.min.js"></script>
    <script src="vendor/vega-embed.min.js"></script>
</head>
<body>
<header>
    <h1>Diary Dashboard</h1>
    <div id="kpis"></div>
    <div id="filters"></div>
</header>
<main>
    <section id="mood-time"></section>
    <section id="writing"></section>
    <section id="tags"></section>
    <section id="calendar"></section>
    <section id="highlights"></section>
    <section id="entries"></section>
</main>
<script id="diary-data" type="application/json">{{ data_json }}</script>
<script type="module" src="app.js"></script>
</body>
</html>
```

Single-File Variant (inline assets)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8"/>
    <title>Diary Dashboard</title>
    <style>
        /* inline style.css content here */
    </style>
    <script>
        // inline vega.min.js
    </script>
    <script>
        // inline vega-lite.min.js
    </script>
    <script>
        // inline vega-embed.min.js
    </script>
</head>
<body>
<header>
    <h1>Diary Dashboard</h1>
    <div id="kpis"></div>
    <div id="filters"></div>
</header>
<main>
    <section id="mood-time"></section>
    <section id="writing"></section>
    <section id="tags"></section>
    <section id="calendar"></section>
    <section id="highlights"></section>
    <section id="entries"></section>
</main>
<script id="diary-data" type="application/json">{{ data_json }}</script>
<script type="module" src="app.js"></script>
</body>
</html>
```

Optional CDN Use (not recommended for strict offline/privacy; use only if acceptable):

```html
<!-- Replace local vendor scripts with CDN versions -->
<script src="https://cdn.jsdelivr.net/npm/vega@5"></script>
<script src="https://cdn.jsdelivr.net/npm/vega-lite@5"></script>
<script src="https://cdn.jsdelivr.net/npm/vega-embed@6"></script>
```

If you use CDN links, note they make network requests and could leak usage metadata. Prefer local copies for
reproducible, offline snapshots.

JavaScript Responsibilities (app.js)

- Parse JSON: `const data = JSON.parse(document.getElementById('diary-data').textContent);`
- Render KPI summary from `data.metadata` and `data.stats.mood.average` etc.
- Define Vega-Lite specs referencing precomputed arrays, e.g.:

```js
vegaEmbed('#mood-time', {
    data: {values: data.stats.mood.daily},
    mark: 'line',
    encoding: {
        x: {field: 'date', type: 'temporal'},
        y: {field: 'avg', type: 'quantitative'}
    }
});
```

- Populate tables (tag usage, mood distribution) with simple DOM or template literals.
- Filtering (Phase 2+): apply in-memory filtering to `data.entries` and regenerate derived views (or precompute multiple
  ranges in Rust if necessary).

CSS Guidance

- Use CSS Grid for layout: header, main grid with sections.
- Ensure readable in dark/light (basic variables: `--bg`, `--fg`, `--accent`).
- Avoid large dependencies; keep file under ~10KB.

Accessibility

- All charts need textual fallback summary (e.g., `<figcaption>` describing trend).
- Use semantic elements: `<section>`, `<header>`, `<table>`.

Performance Tips

- Limit DOM nodes: do not render full note text by default; toggle expansion.
- Use event delegation if interactive filtering added.

Graceful Degradation

- If `data.stats.mood.average === null`, hide numeric mood chart and display message: "No numeric mood data available.".

Testing (Manual)

- Open `index.html` in a browser (no server) and verify charts render.
- Browser console should show no errors.

Potential Enhancements (Later)

- Add client-side date range selector (slice arrays by date string).
- Add tag search box.
- Add local storage preference for hidden sections.
