#[cfg(test)]
mod tests {
    use color_eyre::Result;
    use similar_asserts::assert_eq;

    use daylio_tools::{
        Daylio, DaylioCustomMood, DaylioDayEntry, DaylioMetadata, DaylioTag, Diary,
        load_daylio_backup, load_daylio_pdf,
    };

    #[test]
    fn daylio_format() -> Result<()> {
        let actual = load_daylio_backup("tests/data/official/english.daylio".as_ref())?;

        let mut f = std::fs::File::open("tests/data/official/english.json")?;
        let expected = serde_json::from_reader::<_, Daylio>(&mut f)?;

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    /// This test shows information lost when converting from PDF to JSON.
    /// This is not so bad! The PDF format is not meant to be machine-readable.
    fn pdf_format_english() -> Result<()> {
        let actual = load_daylio_pdf("tests/data/official/english.pdf".as_ref())?;
        let expected = expected_pdf(false);

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn pdf_format_french() -> Result<()> {
        let actual = load_daylio_pdf("tests/data/official/french.pdf".as_ref())?;
        let expected = expected_pdf(true);

        assert_eq!(actual, expected);

        Ok(())
    }

    fn expected_pdf(french: bool) -> Diary {
        let rad_name = if french { "super" } else { "rad" };
        let meh_name = if french { "mouais" } else { "meh" };
        let awful_name = if french { "horrible" } else { "awful" };

        let expected_moods = vec![
            DaylioCustomMood {
                id: 1,
                custom_name: rad_name.to_owned(),
                mood_group_id: 1,
                mood_group_order: 1,
                icon_id: 1,
                predefined_name_id: 1,
                state: 0,
                created_at: 0,
            },
            DaylioCustomMood {
                id: 3,
                custom_name: meh_name.to_owned(),
                mood_group_id: 3,
                mood_group_order: 1,
                icon_id: 3,
                predefined_name_id: 3,
                state: 0,
                created_at: 0,
            },
            DaylioCustomMood {
                id: 5,
                custom_name: awful_name.to_owned(),
                mood_group_id: 5,
                mood_group_order: 1,
                icon_id: 5,
                predefined_name_id: 5,
                state: 0,
                created_at: 0,
            },
            // Unfortunately, the PDF format does not contain the mood group id, so it is guessed
            // In this case, the guess is wrong
            DaylioCustomMood {
                id: 6,
                custom_name: "null".to_owned(),
                mood_group_id: 3,
                mood_group_order: 1,
                icon_id: 3,
                predefined_name_id: -1,
                state: 0,
                created_at: 0,
            },
        ];

        let daylio = Daylio {
            version: 15,
            custom_moods: expected_moods,
            tags: vec![
                DaylioTag {
                    id: 1,
                    name: "exercice".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 1,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 2,
                    name: "famille".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 2,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 3,
                    name: "films".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 3,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 4,
                    name: "manger sain".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 4,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 5,
                    name: "ménage".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 5,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 6,
                    name: "rendez vous".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 6,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 7,
                    name: "shopping".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 7,
                    state: 0,
                    id_tag_group: 0,
                },
                DaylioTag {
                    id: 8,
                    name: "sport".to_owned(),
                    created_at: 0,
                    icon: 1,
                    order: 8,
                    state: 0,
                    id_tag_group: 0,
                },
            ],
            day_entries: vec![
                DaylioDayEntry {
                    id: 5,
                    minute: 0,
                    hour: 20,
                    day: 16,
                    month: 4,
                    year: 2015,
                    datetime: 1431806400000,
                    time_zone_offset: 0,
                    mood: 6,
                    // the line breaks are a bit off, though partially corrected by our heuristics
                    note: "No tag\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but its body is really long".to_owned(),
                    note_title: String::new(),
                    tags: vec![],
                    assets: vec![],
                },
                DaylioDayEntry {
                    id: 4,
                    minute: 0,
                    hour: 20,
                    day: 4,
                    month: 0,
                    year: 2023,
                    datetime: 1672862400000,
                    time_zone_offset: 0,
                    mood: 5,
                    note: String::new(),
                    note_title: String::new(),
                    tags: vec![3, 4, 5, 7],
                    assets: vec![],
                },
                DaylioDayEntry {
                    id: 3,
                    minute: 20,
                    hour: 22,
                    day: 11,
                    month: 0,
                    year: 2023,
                    datetime: 1673475600000,
                    time_zone_offset: 0,
                    mood: 3,
                    note: "Hey, here's a note with\nLinebreaks!\nBecause I love breaking parsers"
                        .to_owned(),
                    note_title: String::new(),
                    tags: vec![4],
                    assets: vec![],
                },
                DaylioDayEntry {
                    id: 2,
                    minute: 59,
                    hour: 9,
                    day: 24,
                    month: 0,
                    year: 2023,
                    datetime: 1674554340000,
                    time_zone_offset: 0,
                    mood: 1,
                    note: "Note title\nNote body".to_owned(), // we lose separation between title and body
                    note_title: String::new(),
                    tags: vec![1, 2, 5, 6, 8],
                    assets: vec![],
                },
                DaylioDayEntry {
                    id: 1,
                    minute: 36,
                    hour: 11,
                    day: 24,
                    month: 0,
                    year: 2023,
                    datetime: 1674560160000, // we lose some precision here
                    time_zone_offset: 0,
                    mood: 5,
                    note: String::new(),
                    note_title: String::new(),
                    tags: vec![],
                    assets: vec![],
                },
            ],
            metadata: DaylioMetadata {
                number_of_entries: 5,
                ..Default::default()
            },
            ..core::default::Default::default()
        };

        // instead of using this from function, we should directly write the Diary struct
        // but I'm too lazy to convert it right now
        let mut expected = Diary::from(daylio);
        for tag in &mut expected.tags {
            tag.icon_id = None; // icon ids are not stored in the PDF
        }
        for mood in &mut expected.moods {
            mood.icon_id = None; // icon ids are not stored in the PDF
            // restore expected wellbeing values
            mood.wellbeing_value = Some(mood.wellbeing_value.unwrap() / 100);
        }

        expected
    }
}
