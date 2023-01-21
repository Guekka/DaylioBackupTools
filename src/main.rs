mod daylio;

use crate::daylio::{CustomMood, DayEntry, Daylio, Tag};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use color_eyre::eyre::{Result, WrapErr};
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

struct IdGenerator {
    offset: i64,
    current: i64,
}

impl IdGenerator {
    fn new(offset: i64) -> Self {
        Self { offset, current: 0 }
    }

    fn next(&mut self) -> i64 {
        self.current += self.offset;
        self.current
    }
}
fn change_mood_id(day_entries: &mut [DayEntry], mood: &mut CustomMood, new_id: i64) {
    for entry in day_entries {
        if entry.mood == mood.id {
            entry.mood = new_id;
        }
    }
    mood.id = new_id
}

fn change_tag_id(day_entries: &mut [DayEntry], tag: &mut Tag, new_id: i64) {
    for entry in day_entries {
        for i in 0..entry.tags.len() {
            if entry.tags[i] == tag.id {
                entry.tags[i] = new_id;
                break;
            }
        }
    }
    tag.id = new_id;
}

fn is_duplicate_mood(mood1: &CustomMood, mood2: &CustomMood) -> bool {
    mood1.custom_name.to_lowercase() == mood2.custom_name.to_lowercase()
        && mood1.icon_id == mood2.icon_id
        && mood1.mood_group_id == mood2.mood_group_id
}

fn is_duplicate_tag(tag1: &Tag, tag2: &Tag) -> bool {
    tag1.name.to_lowercase() == tag2.name.to_lowercase() && tag1.icon == tag2.icon
}

fn merge(mut daylio1: Daylio, mut daylio2: Daylio) -> Daylio {
    const BIG_OFFSET: u64 = 1000;

    // first_pass: make sure we don't have any duplicates id
    let mut id_generator = IdGenerator::new(BIG_OFFSET as i64);
    for daylio in [&mut daylio1, &mut daylio2].iter_mut() {
        for mood in &mut daylio.custom_moods {
            change_mood_id(&mut daylio.day_entries, mood, id_generator.next());
        }

        for tag in &mut daylio.tags {
            change_tag_id(&mut daylio.day_entries, tag, id_generator.next());
        }
    }

    let mut merged = daylio1.clone();
    merged
        .custom_moods
        .append(&mut daylio2.custom_moods.clone());
    merged.tags.append(&mut daylio2.tags.clone());
    merged.day_entries.append(&mut daylio2.day_entries.clone());

    // second_pass: make sure we don't have any duplicate

    // for moods
    merged
        .custom_moods
        .sort_by_key(|x| (x.custom_name.to_lowercase(), x.icon_id));

    for i in 1..merged.custom_moods.len() {
        if is_duplicate_mood(&merged.custom_moods[i - 1], &merged.custom_moods[i]) {
            let new_id = merged.custom_moods[i - 1].id;
            change_mood_id(&mut merged.day_entries, &mut merged.custom_moods[i], new_id);
            merged.custom_moods[i].id = -1; // mark for deletion
        }
    }

    merged.custom_moods.retain(|mood| mood.id != -1);

    // for tags
    merged.tags.sort_by_key(|x| (x.name.to_lowercase(), x.icon));

    for i in 1..merged.tags.len() {
        if is_duplicate_tag(&merged.tags[i - 1], &merged.tags[i]) {
            let new_id = merged.tags[i - 1].id;
            change_tag_id(&mut merged.day_entries, &mut merged.tags[i], new_id);
            merged.tags[i].id = -1; // mark for deletion
        }
    }

    merged.tags.retain(|tag| tag.id != -1);

    // for entries
    merged
        .day_entries
        .sort_by_key(|x| (x.datetime, x.year, x.month));

    for i in 1..merged.day_entries.len() {
        // we do not want to lose any data, so they need to be exactly the same
        if merged.day_entries[i - 1] == merged.day_entries[i] {
            merged.day_entries[i].id = -1; // mark for deletion
        }
    }

    merged.day_entries.retain(|entry| entry.id != -1);

    // finally, sort by date and update ids
    merged
        .custom_moods
        .sort_by_key(|x| (x.mood_group_id, x.created_at));
    merged.tags.sort_by_key(|x| x.created_at);
    merged
        .day_entries
        .sort_by_key(|x| (x.datetime, x.year, x.month));

    // fix: sometimes custom moods have a custom
    // name and a predefined name
    // we keep custom name and remove predefined name
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id != -1 && !mood.custom_name.is_empty() {
            mood.predefined_name_id = -1;
        }
    }

    // ids start at 1
    let mut id_generator = IdGenerator::new(1);
    // first handle predefined moods
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id != -1 {
            change_mood_id(&mut merged.day_entries, mood, id_generator.next());
        }
    }

    // then handle custom moods
    // order is important, so we need to sort by mood_group_id and predefined comes first
    merged
        .custom_moods
        .sort_by_key(|x| (x.mood_group_id, -x.predefined_name_id));
    for mood in merged.custom_moods.iter_mut() {
        if mood.predefined_name_id == -1 {
            change_mood_id(&mut merged.day_entries, mood, id_generator.next());
        }
    }

    // each mood_group_id has an order, so we need to update it
    for i in 0..merged.custom_moods.len() {
        if i == 0
            || merged.custom_moods[i].mood_group_id != merged.custom_moods[i - 1].mood_group_id
        {
            merged.custom_moods[i].mood_group_order = 0;
        } else {
            merged.custom_moods[i].mood_group_order =
                merged.custom_moods[i - 1].mood_group_order + 1;
        }
    }

    let mut id_generator = IdGenerator::new(1);
    for (i, mut tag) in merged.tags.iter_mut().enumerate() {
        change_tag_id(&mut merged.day_entries, tag, id_generator.next());
        tag.order = i as i64;
    }

    let mut id_generator = IdGenerator::new(1);
    for entry in merged.day_entries.iter_mut() {
        entry.id = id_generator.next();
    }

    // update metadata
    merged.metadata.number_of_entries = merged.day_entries.len() as i64;
    merged.metadata.number_of_photos =
        daylio1.metadata.number_of_photos + daylio2.metadata.number_of_photos;
    merged.metadata.photos_size = daylio1.metadata.photos_size + daylio2.metadata.photos_size;

    merged
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
