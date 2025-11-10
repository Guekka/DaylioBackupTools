// filepath: /home/edgar/code/daylio_tools/tests/dashboard_cli.rs
use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn generate_dashboard_basic() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let dir = tempdir()?;
    let out_dir = dir.path().join("dash");
    let input = "localdata/sample.md"; // existing sample diary

    let mut cmd = Command::cargo_bin("daylio_tools")?;
    cmd.arg("generate-dashboard")
        .arg("--input")
        .arg(input)
        .arg("--out-dir")
        .arg(&out_dir);
    cmd.assert().success();

    // Check files
    let index = out_dir.join("index.html");
    let data = out_dir.join("data.json");
    let app = out_dir.join("app.js");
    let style = out_dir.join("style.css");

    assert!(index.exists(), "index.html missing");
    assert!(data.exists(), "data.json missing");
    assert!(app.exists(), "app.js missing");
    assert!(style.exists(), "style.css missing");

    let content = fs::read_to_string(data)?;
    let json: Value = serde_json::from_str(&content)?;
    assert_eq!(json.get("version").and_then(|v| v.as_str()), Some("1"));
    assert!(json.get("stats").is_some());

    Ok(())
}
