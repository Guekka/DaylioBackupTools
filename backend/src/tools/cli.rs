use crate::{
    DayEntryComparisonPolicy, load_daylio_backup, load_daylio_json, load_diary, merge,
    store_daylio_backup, store_daylio_json, store_diary,
};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ToolCommands {
    /// Merge multiple Daylio backups into one
    Merge {
        /// Input files
        #[arg(required = true, num_args = 2..)]
        input: Vec<PathBuf>,
        /// Output file
        output: PathBuf,
    },
    /// Extract the JSON content of a Daylio backup
    Extract {
        /// Input file
        input: PathBuf,
        /// Output file
        output: PathBuf,
    },
    /// Pack a JSON-formatted Daylio into a backup
    Pack {
        /// Input file
        input: PathBuf,
        /// Output file
        output: PathBuf,
    },
    /// Convert a diary file between different formats
    Convert {
        /// Input file
        input: PathBuf,
        /// Output file
        output: PathBuf,
    },
}

pub fn process_command(tool_commands: ToolCommands) -> color_eyre::Result<()> {
    match tool_commands {
        ToolCommands::Merge {
            input: inputs,
            output,
        } => {
            let mut reference = load_diary(&inputs[0])?;

            for path in inputs.iter().skip(1) {
                let other = load_diary(path)?;
                println!(
                    "Merging {:#?} into {:#?}\nMergee has {} entries, reference has {} entries",
                    path,
                    inputs[0],
                    other.day_entries.len(),
                    reference.day_entries.len()
                );
                // TODO: make policy configurable
                reference = merge(reference, other, DayEntryComparisonPolicy::Contained)?;
                println!(
                    "Merged into {:#?} with {} entries",
                    inputs[0],
                    reference.day_entries.len()
                );
            }

            let word_count = reference
                .day_entries
                .iter()
                .map(|entry| entry.note.split_whitespace().count())
                .sum::<usize>();

            store_diary(reference, &output)?;
            println!("Wrote merged file to {output:#?}");
            println!("Approximately {word_count} words were written. Congrats!");
        }
        ToolCommands::Extract { input, output } => {
            let daylio = load_daylio_backup(&input)?;
            store_daylio_json(&daylio, &output)?;
        }
        ToolCommands::Pack { input, output } => {
            let daylio = load_daylio_json(&input)?;
            store_daylio_backup(&daylio, &output)?;
        }
        ToolCommands::Convert { input, output } => {
            let diary = load_diary(&input)?;
            store_diary(diary, &output)?;
        }
    }
    Ok(())
}
