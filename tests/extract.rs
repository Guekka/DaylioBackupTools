#[cfg(test)]
mod tests {
    use color_eyre::Result;
    use daylio_tools::{
        load_daylio_backup, load_daylio_pdf, CustomMood, DayEntry, Daylio, Metadata, Tag,
    };
    use similar_asserts::assert_eq;

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
        let expected = expected_pdf();

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn pdf_format_french() -> Result<()> {
        let actual = load_daylio_pdf("tests/data/official/french.pdf".as_ref())?;
        let expected = expected_pdf();

        assert_eq!(actual, expected);

        Ok(())
    }

    fn expected_pdf() -> Daylio {
        let mut expected_moods = Daylio::default().custom_moods;
        // Unfortunately, the PDF format does not contain the mood group id, so it is guessed
        // In this case, the guess is wrong
        expected_moods.insert(
            3,
            CustomMood {
                id: 6,
                custom_name: "NULL".to_owned(),
                mood_group_id: 3,
                mood_group_order: 1,
                icon_id: 0,
                predefined_name_id: -1,
                state: 0,
                created_at: 0,
            },
        );

        Daylio {
            version: 15,
            custom_moods: expected_moods,
            tags: vec![
                Tag {
                    id: 1,
                    name: "exercice".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 1,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 2,
                    name: "famille".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 2,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 3,
                    name: "films".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 3,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 4,
                    name: "manger sain".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 4,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 5,
                    name: "ménage".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 5,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 6,
                    name: "rendez vous".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 6,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 7,
                    name: "shopping".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 7,
                    state: 0,
                    id_tag_group: 0,
                },
                Tag {
                    id: 8,
                    name: "sport".to_owned(),
                    created_at: 0,
                    icon: 0,
                    order: 8,
                    state: 0,
                    id_tag_group: 0,
                },
            ],
            day_entries: vec![
                DayEntry {
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
                DayEntry {
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
                    tags: vec![
                        1,
                        5,
                        8,
                        2,
                        6,
                    ],
                    assets: vec![],
                },
                DayEntry {
                    id: 3,
                    minute: 20,
                    hour: 22,
                    day: 11,
                    month: 0,
                    year: 2023,
                    datetime: 1673475600000,
                    time_zone_offset: 0,
                    mood: 3,
                    note: "Hey, here's a note with\nLinebreaks!\nBecause I love breaking parsers".to_owned(),
                    note_title: String::new(),
                    tags: vec![
                        4,
                    ],
                    assets: vec![],
                },
                DayEntry {
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
                    tags: vec![
                        4, 5, 3, 7,
                    ],
                    assets: vec![],
                },
                DayEntry {
                    id: 5,
                    minute: 0,
                    hour: 20,
                    day: 16,
                    month: 4,
                    year: 2015,
                    datetime: 1431806400000,
                    time_zone_offset: 0,
                    mood: 6,
                    // the line breaks are a bit off
                    note: "No tag\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but\nits body is really long\nThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but\nits body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but\nits body is really long\nThis is an old note. It has no title, but its body is really long\nThis is an old note. It has no title, but its body is really longThis is an old note. It has no title, but\nits body is really long".to_owned(),
                    note_title: String::new(),
                    tags: vec![],
                    assets: vec![],
                },
            ],
            metadata: Metadata {
                number_of_entries: 5,
                ..Default::default()
            },
            ..core::default::Default::default()
        }
    }
}
