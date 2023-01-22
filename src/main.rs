use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use color_eyre::eyre::{Result, WrapErr};
use daylio_tools::{merge, Daylio};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use zip::write::FileOptions;
use zip::ZipWriter;

fn load_daylio_backup(path: &Path) -> Result<Daylio> {
    let file = File::open(path)?;

    let mut archive = zip::ZipArchive::new(file)?;
    let mut file = archive.by_name("backup.daylio")?;

    let mut data = String::new();
    file.read_to_string(&mut data)?;
    data = data.replace('\n', "");

    let data = BASE64.decode(data)?;

    serde_json::from_slice(&data).wrap_err("Failed to parse Daylio backup")
}

fn store_daylio_backup(daylio: &Daylio, path: &Path) -> Result<()> {
    let file = File::create(path)?;

    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let json = serde_json::to_string_pretty(daylio)?;

    // write temp json file
    // append to path
    let mut path = path.to_path_buf();
    path.set_extension("json");
    let mut f2 = File::create(&path)?;
    f2.write_all(json.as_bytes())?;

    let data = BASE64.encode(json.as_bytes());

    archive.start_file("backup.daylio", options)?;
    archive.write_all(data.as_bytes())?;
    archive.finish()?;

    Ok(())
}

///     Merges two daylio json files into one.
///     We assume the files have version 15, but this is not checked.
///     We keep everything from the first file, and add the new entries from the second file
fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        println!("Usage: main <main.daylio> <new.daylio> [<out.daylio>]");
        std::process::exit(1);
    }

    let input1 = Path::new(&args[1]);
    let input2 = Path::new(&args[2]);

    let old = load_daylio_backup(input1)?;
    let new = load_daylio_backup(input2)?;

    if old.version != 15 || new.version != 15 {
        println!("Warning: version is not 15. This may not work.");
    }

    let out = if args.len() == 4 {
        Path::new(&args[3])
    } else {
        Path::new("out.daylio")
    };

    let merged = merge(old, new);
    store_daylio_backup(&merged, out)?;

    Ok(())
}
