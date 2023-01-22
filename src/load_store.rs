use crate::Daylio;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;
use zip::write::FileOptions;
use zip::ZipWriter;

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

pub fn store_daylio_backup(daylio: &Daylio, path: &Path) -> Result<()> {
    let file = File::create(path)?;

    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
