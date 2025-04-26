use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, WrapErr, eyre};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::Daylio;
use crate::analyze_pdf::ProcessedPdf;
use crate::parse_md::load_md;

pub fn load_daylio_backup(path: &Path) -> Result<Daylio> {
    let file = File::open(path)?;

    let mut archive = zip::ZipArchive::new(file)?;
    let mut file = archive.by_name("backup.daylio")?;

    let mut data = String::new();
    file.read_to_string(&mut data)?;
    data = data.replace('\n', "");

    let data = BASE64.decode(data)?;

    serde_json::from_slice(&data).wrap_err("Failed to parse Daylio backup")
}

pub fn load_daylio_json(path: &Path) -> Result<Daylio> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    serde_json::from_str(&data).wrap_err("Failed to parse Daylio JSON")
}

pub fn load_daylio_pdf(path: &Path) -> Result<Daylio> {
    crate::parse_pdf::parse_pdf(path)
        .and_then(TryInto::<ProcessedPdf>::try_into)
        .and_then(TryInto::<Daylio>::try_into)
}

pub fn load_daylio(path: &Path) -> Result<Daylio> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().wrap_err("Unknown file extension")?;
        match ext.to_lowercase().as_ref() {
            "daylio" => load_daylio_backup(path),
            "json" => load_daylio_json(path),
            "pdf" => load_daylio_pdf(path),
            "md" => load_md(path),

            _ => Err(eyre!("Unknown file extension")),
        }
    } else {
        Err(eyre!("Missing file extension"))
    }
}

pub fn store_daylio_backup(daylio: &Daylio, path: &Path) -> Result<()> {
    let file = File::create(path)?;

    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let json = serde_json::to_string_pretty(daylio)?;

    let data = BASE64.encode(json.as_bytes());

    archive.start_file("backup.daylio", options)?;
    archive.write_all(data.as_bytes())?;
    archive.finish()?;

    Ok(())
}

pub fn store_daylio_json(daylio: &Daylio, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(daylio)?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

pub fn store_daylio_md(daylio: &Daylio, path: &Path) -> Result<()> {
    let mut file = File::create(path)?;
    let tag_map: HashMap<i64, String> = daylio
        .tags
        .iter()
        .map(|tag| (tag.id, tag.name.clone()))
        .collect();

    let mood_map: HashMap<i64, String> = daylio
        .custom_moods
        .iter()
        .map(|mood| (mood.id, mood.custom_name.clone()))
        .collect();

    for entry in &daylio.day_entries {
        writeln!(
            file,
            "[{:04}-{:02}-{:02} {:02}:{:02}]",
            entry.year, entry.month, entry.day, entry.hour, entry.minute
        )?;
        writeln!(file, "{{{}}}", mood_map.get(&entry.mood).unwrap())?;
        writeln!(
            file,
            "{}",
            entry
                .tags
                .iter()
                .map(|tag| format!("#{}", tag_map.get(tag).unwrap()))
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(file, "{}\n", entry.note)?;
    }

    Ok(())
}

pub fn store_daylio(daylio: &Daylio, path: &Path) -> Result<()> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().wrap_err("Unknown file extension")?;
        match ext.to_lowercase().as_ref() {
            "daylio" => store_daylio_backup(daylio, path),
            "json" => store_daylio_json(daylio, path),
            "md" => store_daylio_md(daylio, path),
            _ => Err(eyre!("Unknown file extension")),
        }
    } else {
        Err(eyre!("Missing file extension"))
    }
}
