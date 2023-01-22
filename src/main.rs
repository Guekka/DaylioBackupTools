use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use color_eyre::eyre::{Result, WrapErr};
use daylio_tools::{merge, Daylio};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
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

fn load_daylio_json(path: &Path) -> Result<Daylio> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;

    serde_json::from_str(&data).wrap_err("Failed to parse Daylio JSON")
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

fn store_daylio_json(daylio: &Daylio, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(daylio)?;

    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

enum Command {
    Merge {
        input: Vec<PathBuf>,
        output: PathBuf,
    },
    Anonymize {
        input: PathBuf,
        output: PathBuf,
    },
    Extract {
        input: PathBuf,
        output: PathBuf,
    },
    Pack {
        input: PathBuf,
        output: PathBuf,
    },
}

fn parse_args() -> Result<Command> {
    let args: Vec<String> = env::args().collect();

    let command = args.get(1).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "Missing command. Usage: daylio-tools <command> <input(s)> <output>"
        )
    })?;

    match command.as_str() {
        "merge" => {
            let inputs = args.iter().skip(2).map(PathBuf::from).collect::<Vec<_>>();

            if inputs.len() < 2 {
                return Err(color_eyre::eyre::eyre!("Missing input files"));
            }

            let output = inputs
                .last()
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing output path"))?
                .to_path_buf();

            Ok(Command::Merge {
                input: inputs,
                output,
            })
        }
        "anonymize" => {
            let input = args
                .get(2)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing input path"))?;
            let output = args
                .get(3)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing output path"))?;

            let input = PathBuf::from(input);
            let output = PathBuf::from(output);

            Ok(Command::Anonymize { input, output })
        }
        "extract" => {
            let input = args
                .get(2)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing input path"))?;
            let output = args
                .get(3)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing output path"))?;

            let input = PathBuf::from(input);
            let output = PathBuf::from(output);

            Ok(Command::Extract { input, output })
        }
        "pack" => {
            let input = args
                .get(2)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing input path"))?;
            let output = args
                .get(3)
                .ok_or_else(|| color_eyre::eyre::eyre!("Missing output path"))?;

            let input = PathBuf::from(input);
            let output = PathBuf::from(output);

            Ok(Command::Pack { input, output })
        }
        _ => Err(color_eyre::eyre::eyre!("Unknown command")),
    }
}

/// Merges two daylio json files into one.
/// We assume the files have version 15, but this is not checked.
/// We keep everything from the first file, and add the new entries from the other files
fn main() -> Result<()> {
    color_eyre::install()?;

    let command = parse_args()?;

    match command {
        Command::Merge { input, output } => {
            let mut daylio = load_daylio_backup(&input[0])?;

            for path in input.iter().skip(1) {
                let other = load_daylio_backup(path)?;
                daylio = merge(daylio, other);
            }
            store_daylio_backup(&daylio, &output)?;
        }
        Command::Anonymize { input, output } => {
            let mut daylio = load_daylio_backup(&input)?;
            daylio_tools::anonymize(&mut daylio);
            store_daylio_backup(&daylio, &output)?;
        }
        Command::Extract { input, output } => {
            let daylio = load_daylio_backup(&input)?;
            store_daylio_json(&daylio, &output)?;
        }
        Command::Pack { input, output } => {
            let daylio = load_daylio_json(&input)?;
            store_daylio_backup(&daylio, &output)?;
        }
    }

    Ok(())
}
