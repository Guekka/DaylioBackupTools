// filepath: /home/edgar/code/daylio_tools/src/dashboard/export.rs
use chrono::Utc;
use color_eyre::Result;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::dashboard::data::DashboardData;

pub fn write_bundle(data: &DashboardData, out_dir: &Path, single_file: bool) -> Result<()> {
    fs::create_dir_all(out_dir)?;
    // data.json
    let json = serde_json::to_string_pretty(data)?;
    let mut f = File::create(out_dir.join("data.json"))?;
    f.write_all(json.as_bytes())?;

    // style.css
    let css = r#":root{font-family:system-ui,sans-serif;}body{margin:1rem;}header{margin-bottom:2rem;}section{margin:1rem 0;}table{border-collapse:collapse;}td,th{border:1px solid #ccc;padding:0.25rem;}figure{margin:0;}"#;
    File::create(out_dir.join("style.css"))?.write_all(css.as_bytes())?;

    // app.js using Vega-Lite
    let app_js = r#"const data = JSON.parse(document.getElementById('diary-data').textContent);
function fmt(x){return x==null?'-':(typeof x==='number'?x.toFixed(2):x);}

// KPI summary
const kpis = document.getElementById('kpis');
let avg = data.stats.mood.average; let prev = data.stats.mood.previous_period_average; let delta = (avg!=null && prev!=null)?(avg-prev):null;
kpis.innerHTML = `<div class='kpi'><strong>Average mood:</strong> ${fmt(avg)} ${delta!=null?`(Δ ${fmt(delta)})`:''}</div>
<div class='kpi'><strong>Entries:</strong> ${data.metadata.total_entries}</div>
<div class='kpi'><strong>Days logged:</strong> ${data.metadata.total_days_logged}</div>
<div class='kpi'><strong>Words total:</strong> ${data.metadata.word_total}</div>`;

// Highlights
const highlights = document.getElementById('highlights');
if (data.highlights && data.highlights.length) {
  const items = data.highlights.map(h => `<li><strong>${h.kind}:</strong> ${h.message}</li>`).join('');
  highlights.innerHTML = `<h2>Highlights</h2><ul>${items}</ul>`;
} else { highlights.innerHTML = '<h2>Highlights</h2><p>No highlights.</p>'; }

// Mood over time chart (Vega-Lite)
const moodDiv = document.getElementById('mood-time');
moodDiv.innerHTML = `<h2>Mood Over Time</h2><div id='mood-chart'></div>`;
if (data.stats.mood.daily && data.stats.mood.daily.some(d=>d.avg!=null)) {
  const values = data.stats.mood.daily.filter(d=>d.avg!=null);
  const spec = {
    $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
    width: 700,
    height: 250,
    data: { values },
    mark: { type: 'line', point: true },
    encoding: {
      x: { field: 'date', type: 'temporal', title: 'Date' },
      y: { field: 'avg', type: 'quantitative', title: 'Average Mood' }
    }
  };
  vegaEmbed('#mood-chart', spec, {actions:false});
} else {
  moodDiv.insertAdjacentHTML('beforeend', '<p>No numeric mood data available.</p>');
}

// Words per day chart (Vega-Lite)
const writing = document.getElementById('writing');
writing.innerHTML = `<h2>Words Per Day</h2><div id='words-chart'></div>`;
if (data.stats.writing.words_daily && data.stats.writing.words_daily.length) {
  const spec = {
    $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
    width: 700,
    height: 250,
    data: { values: data.stats.writing.words_daily },
    mark: 'bar',
    encoding: {
      x: { field: 'date', type: 'temporal', title: 'Date' },
      y: { field: 'words', type: 'quantitative', title: 'Words' }
    }
  };
  vegaEmbed('#words-chart', spec, {actions:false});
} else {
  writing.insertAdjacentHTML('beforeend', '<p>No writing data.</p>');
}

// Tag usage chart + table (Vega-Lite)
const tags = document.getElementById('tags');
tags.innerHTML = `<h2>Tag Usage</h2><div id='tags-chart'></div><div id='table-tags'></div>`;
if (data.stats.tags.usage && data.stats.tags.usage.length) {
  const values = data.stats.tags.usage.slice(0,40);
  const spec = {
    $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
    width: 700,
    height: 20*values.length,
    data: { values },
    mark: 'bar',
    encoding: {
      y: { field: 'tag', type: 'nominal', sort: '-x', title: 'Tag' },
      x: { field: 'count', type: 'quantitative', title: 'Count' }
    }
  };
  vegaEmbed('#tags-chart', spec, {actions:false});
  let rows = data.stats.tags.usage.map(u=>`<tr><td>${u.tag}</td><td>${u.count}</td><td>${u.first}</td><td>${u.last}</td></tr>`).join('');
  document.getElementById('table-tags').innerHTML = `<table><thead><tr><th>Tag</th><th>Count</th><th>First</th><th>Last</th></tr></thead><tbody>${rows}</tbody></table>`;
} else { tags.insertAdjacentHTML('beforeend', '<p>No tags.</p>'); }

// Temporal patterns (textual)
const temporal = document.getElementById('temporal');
const wk = data.stats.temporal.weekday_mood.map(w => `W${w.weekday}: avg=${fmt(w.avg)} (n=${w.samples})`).join('<br>');
const hr = data.stats.temporal.hour_entries.map(h => `H${h.hour}: entries=${h.entries}`).join('<br>');
temporal.innerHTML = `<h2>Temporal Patterns</h2><div class='grid-2'><div><h3>Weekdays</h3>${wk}</div><div><h3>Hours</h3>${hr}</div></div>`;

// Mood Combos
const combos = document.getElementById('combos');
const comboLines = data.stats.mood.combos.map(c => `${c.moods.join(' + ')}: ${c.count}`).join('<br>');
combos.innerHTML = `<h2>Mood Combos</h2><div>${comboLines||'None'}</div>`;

// Tag Pairs
const pairs = document.getElementById('pairs');
const pairLines = data.stats.tags.pairs.map(p => `${p.tags[0]} + ${p.tags[1]}: ${p.count}`).join('<br>');
pairs.innerHTML = `<h2>Tag Pairs</h2><div>${pairLines||'None'}</div>`;

// Emerging Tags
const emerging = document.getElementById('emerging');
if (data.stats.tags.emerging && data.stats.tags.emerging.length) {
  const lines = data.stats.tags.emerging.map(e => `${e.tag}: x${fmt(e.growth_factor)} (${e.previous_count} -> ${e.current_count})`).join('<br>');
  emerging.innerHTML = `<h2>Emerging Tags</h2><div>${lines}</div>`;
} else { emerging.innerHTML = '<h2>Emerging Tags</h2><p>No emerging tags detected.</p>'; }
"#;
    File::create(out_dir.join("app.js"))?.write_all(app_js.as_bytes())?;

    // index.html: add Vega CDN scripts
    let html = format!(
        r#"<!DOCTYPE html><html lang='en'><head><meta charset='UTF-8'/><title>Diary Dashboard</title><meta name='viewport' content='width=device-width,initial-scale=1'/><link rel='stylesheet' href='style.css'>
<script src='https://cdn.jsdelivr.net/npm/vega@5'></script>
<script src='https://cdn.jsdelivr.net/npm/vega-lite@5'></script>
<script src='https://cdn.jsdelivr.net/npm/vega-embed@6'></script>
</head><body><header><h1>Diary Dashboard</h1><div id='kpis' class='kpis'></div></header><main><section id='highlights'></section><section id='mood-time'></section><section id='writing'></section><section id='tags'></section><section id='temporal'></section><section id='combos'></section><section id='pairs'></section><section id='emerging'></section><section id='calendar'></section></main><script id='diary-data' type='application/json'>{}</script><script src='app.js' type='module'></script></body></html>"#,
        json
    );
    File::create(out_dir.join("index.html"))?.write_all(html.as_bytes())?;

    if single_file {
        let mut hasher = Sha256::new();
        hasher.update(html.as_bytes());
        let digest = hasher.finalize();
        let sha = format!("{:x}", digest);
        let inline = format!(
            "<!-- generated:{} sha256:{} -->\n{}",
            Utc::now().to_rfc3339(),
            sha,
            html
        );
        File::create(out_dir.join("index.single.html"))?.write_all(inline.as_bytes())?;
    }

    Ok(())
}
