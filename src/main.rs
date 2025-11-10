use std::path::PathBuf;

use color_eyre::eyre::Result;

use clap::{ArgAction, Parser, Subcommand};
use daylio_tools::dashboard::data::PeriodSelector;
use daylio_tools::dashboard::export::write_bundle;
use daylio_tools::dashboard::{DashboardConfig, generate_dashboard_data};
use daylio_tools::{
    DayEntryComparisonPolicy, load_daylio_backup, load_daylio_json, load_diary, merge,
    store_daylio_backup, store_daylio_json, store_diary,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Generate a static dashboard bundle from a diary
    GenerateDashboard {
        /// Input diary file
        #[arg(long = "input")]
        input: PathBuf,
        /// Output directory
        #[arg(long = "out-dir")]
        out_dir: PathBuf,
        /// Period specification (all|last30|last90|ytd|year:YYYY|from:YYYY-MM-DD,to:YYYY-MM-DD)
        #[arg(long = "period", default_value = "all")]
        period: String,
        /// Include note text bodies in output
        #[arg(long = "include-notes", action=ArgAction::SetTrue)]
        include_notes: bool,
        /// Anonymize tag names
        #[arg(long = "anonymize-tags", action=ArgAction::SetTrue)]
        anonymize_tags: bool,
        /// Produce single-file variant
        #[arg(long = "single-file", action=ArgAction::SetTrue)]
        single_file: bool,
        /// Minimum samples for correlations
        #[arg(long = "min-samples", default_value_t = 5)]
        min_samples: usize,
        /// Word threshold for writing streak
        #[arg(long = "word-threshold", default_value_t = 10)]
        word_threshold: usize,
        /// Max mood combos
        #[arg(long = "max-combos", default_value_t = 50)]
        max_combos: usize,
        /// Max tag pairs
        #[arg(long = "max-tag-pairs", default_value_t = 50)]
        max_tag_pairs: usize,
    },
}

fn parse_period(spec: &str) -> color_eyre::Result<PeriodSelector> {
    if spec == "all" {
        return Ok(PeriodSelector::All);
    }
    if spec == "last30" {
        return Ok(PeriodSelector::LastNDays(30));
    }
    if spec == "last90" {
        return Ok(PeriodSelector::LastNDays(90));
    }
    if spec == "ytd" {
        return Ok(PeriodSelector::YearToDate);
    }
    if let Some(rest) = spec.strip_prefix("year:") {
        let y: u32 = rest.parse()?;
        return Ok(PeriodSelector::Year(y));
    }
    if let Some(rest) = spec.strip_prefix("from:") {
        if let Some((from, to)) = rest.split_once(",to:") {
            let from_date = chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d")?;
            let to_date = chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")?;
            return Ok(PeriodSelector::Range {
                from: from_date,
                to: to_date,
            });
        }
    }
    color_eyre::eyre::bail!("Invalid period spec: {spec}")
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Merge {
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
        Commands::Extract { input, output } => {
            let daylio = load_daylio_backup(&input)?;
            store_daylio_json(&daylio, &output)?;
        }
        Commands::Pack { input, output } => {
            let daylio = load_daylio_json(&input)?;
            store_daylio_backup(&daylio, &output)?;
        }
        Commands::Convert { input, output } => {
            let diary = load_diary(&input)?;
            store_diary(diary, &output)?;
        }
        Commands::GenerateDashboard {
            input,
            out_dir,
            period,
            include_notes,
            anonymize_tags,
            single_file,
            min_samples,
            word_threshold,
            max_combos,
            max_tag_pairs,
        } => {
            let diary = load_diary(&input)?;
            println!("Loaded diary with {} entries", diary.day_entries.len());
            let period_sel = parse_period(&period)?;
            let cfg = DashboardConfig {
                period: period_sel.clone(),
                include_notes,
                anonymize_tags,
                single_file,
                min_samples,
                word_threshold,
                max_combos,
                max_tag_pairs,
            };
            let data = generate_dashboard_data(&diary, &cfg);
            if data.entries.is_empty() {
                color_eyre::eyre::bail!("No entries found after applying period filter.");
            }
            write_bundle(&data, &out_dir, single_file)?;
            println!("Dashboard generated at {:?}", out_dir);
        }
    }

    Ok(())
}
