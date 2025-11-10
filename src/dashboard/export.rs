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

    // vendor placeholder minimal JS (no real Vega to keep deps light for MVP)
    let vendor_dir = out_dir.join("vendor");
    fs::create_dir_all(&vendor_dir)?;
    let mut vf = File::create(vendor_dir.join("vega-lite-stub.js"))?;
    vf.write_all(b"window.VEGA_STUB=true;")?;

    // style.css
    let css = r#":root{font-family:system-ui,sans-serif;}body{margin:1rem;}header{margin-bottom:2rem;}section{margin:1rem 0;}table{border-collapse:collapse;}td,th{border:1px solid #ccc;padding:0.25rem;}"#;
    File::create(out_dir.join("style.css"))?.write_all(css.as_bytes())?;

    // app.js
    let app_js = r#"const data = JSON.parse(document.getElementById('diary-data').textContent);
// KPI summary
const kpis = document.getElementById('kpis');
function fmt(x){return x==null?'-':x.toFixed?x.toFixed(2):x;}
let avg = data.stats.mood.average; let prev = data.stats.mood.previous_period_average; let delta = (avg!=null && prev!=null)?(avg-prev):null;
kpis.innerHTML = `<div><strong>Average mood:</strong> ${fmt(avg)} ${delta!=null?`(Δ ${fmt(delta)})`:''}</div>
<div><strong>Entries:</strong> ${data.metadata.total_entries}</div>
<div><strong>Days logged:</strong> ${data.metadata.total_days_logged}</div>
<div><strong>Words total:</strong> ${data.metadata.word_total}</div>`;
// Mood over time (simple)
const moodTime = document.getElementById('mood-time');
if (data.stats.mood.daily.length && data.stats.mood.daily.some(d=>d.avg!=null)) {
  let pts = data.stats.mood.daily.filter(d=>d.avg!=null).map(d=>`${d.date},${d.avg}`);
  moodTime.innerHTML = `<h2>Mood Over Time</h2><pre>${pts.join('\n')}</pre>`;
} else { moodTime.innerHTML = '<h2>Mood Over Time</h2><p>No numeric mood data.</p>'; }
// Words per day
const writing = document.getElementById('writing');
let wpts = data.stats.writing.words_daily.map(d=>`${d.date},${d.words}`);
writing.innerHTML = `<h2>Words Per Day</h2><pre>${wpts.join('\n')}</pre>`;
// Tag usage table
const tags = document.getElementById('tags');
let rows = data.stats.tags.usage.map(u=>`<tr><td>${u.tag}</td><td>${u.count}</td><td>${u.first}</td><td>${u.last}</td></tr>`).join('');
tags.innerHTML = `<h2>Tag Usage</h2><table><thead><tr><th>Tag</th><th>Count</th><th>First</th><th>Last</th></tr></thead><tbody>${rows}</tbody></table>`;
"#;
    File::create(out_dir.join("app.js"))?.write_all(app_js.as_bytes())?;

    // index.html
    let html = format!(
        r#"<!DOCTYPE html><html lang='en'><head><meta charset='UTF-8'/><title>Diary Dashboard</title><meta name='viewport' content='width=device-width,initial-scale=1'/><link rel='stylesheet' href='style.css'></head><body><header><h1>Diary Dashboard</h1><div id='kpis'></div></header><main><section id='mood-time'></section><section id='writing'></section><section id='tags'></section><section id='calendar'></section><section id='highlights'></section></main><script id='diary-data' type='application/json'>{}</script><script src='app.js' type='module'></script></body></html>"#,
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
