use std::fs::File;
use std::io::Read;
use std::io::prelude::*;
use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, WrapErr, eyre};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::models::Diary;
use crate::parse_md::load_md;
use crate::{Daylio, MdMetadata};

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

pub fn load_daylio_pdf(path: &Path) -> Result<Diary> {
    crate::parse_pdf::parse_pdf(path).and_then(TryInto::<Diary>::try_into)
}

pub fn load_diary_json(path: &Path) -> Result<Diary> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    serde_json::from_str(&data).wrap_err("Failed to parse Diary JSON")
}

pub fn load_diary(path: &Path) -> Result<Diary> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().wrap_err("Unknown file extension")?;
        match ext.to_lowercase().as_ref() {
            "daylio" => load_daylio_backup(path).map(Into::<Diary>::into),
            "pdf" => load_daylio_pdf(path),
            "md" => load_md(path),
            "daylio.json" => load_daylio_json(path).map(Into::<Diary>::into),
            "json" => load_diary_json(path),

            _ => Err(eyre!("Unknown file extension")),
        }
    } else {
        Err(eyre!("Missing file extension"))
    }
}

pub fn store_daylio_backup(daylio: &Daylio, path: &Path) -> Result<()> {
    daylio.check_soundness()?;

    let file = File::create(path)?;

    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let json = serde_json::to_string_pretty(&daylio)?;

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

pub fn store_diary_md(mut diary: Diary, path: &Path) -> Result<()> {
    let mut file = File::create(path)?;
    diary.day_entries.sort_unstable_by_key(|entry| entry.date);

    // YAML header
    let metadata = MdMetadata {
        moods: diary.moods.clone(),
        tags: diary.tags.clone(),
    };
    let yaml = serde_yaml::to_string(&metadata)?;
    writeln!(file, "---")?;
    writeln!(file, "{yaml}")?;
    writeln!(file, "---\n")?;

    for entry in diary.day_entries {
        writeln!(file, "{}", &entry.date.format("[%Y-%m-%d %H:%M]"))?;

        let moods_str = entry
            .moods
            .iter()
            .map(|mood| mood.name.clone())
            .collect::<Vec<_>>()
            .join(" / ");

        if !moods_str.is_empty() {
            writeln!(file, "{{{moods_str}}}")?;
        }

        let tags_str = entry
            .tags
            .iter()
            .map(|tag| tag.name.clone())
            .collect::<Vec<_>>()
            .join(",");

        if !tags_str.is_empty() {
            writeln!(file, "#{{{tags_str}}}")?;
        }
        writeln!(file, "{}\n", entry.note)?;
    }

    Ok(())
}

fn store_diary_json(diary: &Diary, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(diary)?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

pub fn store_diary(diary: Diary, path: &Path) -> Result<()> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().wrap_err("Unknown file extension")?;
        match ext.to_lowercase().as_ref() {
            "daylio" => store_daylio_backup(&diary.try_into()?, path),
            "md" => store_diary_md(diary, path),
            "daylio.json" => store_daylio_json(&diary.try_into()?, path),
            "json" => store_diary_json(&diary, path),
            _ => Err(eyre!("Unknown file extension")),
        }
    } else {
        Err(eyre!("Missing file extension"))
    }
}
