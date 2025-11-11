// filepath: /home/edgar/code/daylio_tools/src/dashboard/export.rs
use crate::dashboard::data::DashboardData;
use color_eyre::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

const STYLE_CSS: &str = include_str!("./assets/style.css");
const APP_JS: &str = include_str!("./assets/app.js");
const INDEX_HTML_TMPL: &str = include_str!("./assets/index.html");

pub fn write_bundle(data: &DashboardData, out_dir: &Path, single_file: bool) -> Result<()> {
    fs::create_dir_all(out_dir)?;
    let json = serde_json::to_string_pretty(data)?;
    File::create(out_dir.join("data.json"))?.write_all(json.as_bytes())?;

    // Write static assets
    File::create(out_dir.join("style.css"))?.write_all(STYLE_CSS.as_bytes())?;
    File::create(out_dir.join("app.js"))?.write_all(APP_JS.as_bytes())?;

    // Build index.html from template by replacing the placeholder
    let index_html = INDEX_HTML_TMPL.replace("__EMBED_DATA__", &json);
    File::create(out_dir.join("index.html"))?.write_all(index_html.as_bytes())?;

    if single_file {
        // Inline CSS and JS into the template
        let single_html = INDEX_HTML_TMPL
            .replace(
                "<link rel=\"stylesheet\" href=\"style.css\" />",
                &format!("<style>{}</style>", STYLE_CSS),
            )
            .replace(
                "<script src=\"app.js\" type=\"module\"></script>",
                &format!("<script type='module'>{}</script>", APP_JS),
            )
            .replace("__EMBED_DATA__", &json)
            .replace("Diary Dashboard", "Diary Dashboard (Single)");
        File::create(out_dir.join("index.single.html"))?.write_all(single_html.as_bytes())?;
    }

    Ok(())
}
