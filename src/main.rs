use color_eyre::eyre::{ContextCompat, Result};
use daylio_tools::{load_daylio, merge, store_daylio_backup, store_daylio_json};
use std::env;
use std::path::PathBuf;

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

    let get_single_in_out = || -> Result<(PathBuf, PathBuf)> {
        let input = args
            .get(2)
            .ok_or_else(|| color_eyre::eyre::eyre!("Missing input path"))?;
        let output = args
            .get(3)
            .ok_or_else(|| color_eyre::eyre::eyre!("Missing output path"))?;

        let input = PathBuf::from(input);
        let output = PathBuf::from(output);

        Ok((input, output))
    };

    match command.as_str() {
        "merge" => {
            let mut inputs = args.iter().skip(2).map(PathBuf::from).collect::<Vec<_>>();
            let output = inputs.pop().wrap_err("Missing output file")?; // last one is output

            if inputs.len() < 2 {
                return Err(color_eyre::eyre::eyre!("Missing input files"));
            }

            Ok(Command::Merge {
                input: inputs,
                output,
            })
        }
        "anonymize" => {
            let args = get_single_in_out()?;
            Ok(Command::Anonymize {
                input: args.0,
                output: args.1,
            })
        }
        "extract" => {
            let args = get_single_in_out()?;
            Ok(Command::Extract {
                input: args.0,
                output: args.1,
            })
        }
        "pack" => {
            let args = get_single_in_out()?;
            Ok(Command::Pack {
                input: args.0,
                output: args.1,
            })
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
            let mut daylio = load_daylio(&input[0])?;

            for path in input.iter().skip(1) {
                let other = load_daylio(path)?;
                daylio = merge(daylio, other);
            }
            store_daylio_backup(&daylio, &output)?;
        }
        Command::Anonymize { input, output } => {
            let mut daylio = load_daylio(&input)?;
            daylio_tools::anonymize(&mut daylio);
            store_daylio_backup(&daylio, &output)?;
        }
        Command::Extract { input, output } => {
            let daylio = load_daylio(&input)?;
            store_daylio_json(&daylio, &output)?;
        }
        Command::Pack { input, output } => {
            let daylio = load_daylio(&input)?;
            store_daylio_backup(&daylio, &output)?;
        }
    }

    Ok(())
}
