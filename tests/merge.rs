#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use color_eyre::Result;
    use std::collections::HashSet;

    use daylio_tools::{
        DayEntry, Daylio, DaylioCustomMood, DaylioDayEntry, DaylioTag, Diary, Mood,
        load_daylio_backup, merge,
    };
    use similar_asserts::assert_eq;

    #[test]
    fn duplicates_removal() {
        // duplicated pre-defined mood
        let custom_moods = vec![
            // duplicated pre-defined mood
            DaylioCustomMood {
                id: 2,
                custom_name: "Predefined".to_owned(),
                icon_id: 2,
                predefined_name_id: 1,
                state: 1,
                mood_group_id: 2,
                mood_group_order: 1,
                created_at: 1,
            },
            // next 2: everything differs except name
            DaylioCustomMood {
                id: 3,
                custom_name: "Mood".to_owned(),
                icon_id: 3,
                predefined_name_id: -1,
                state: 0,
                mood_group_id: 3,
                mood_group_order: 2,
                created_at: 2,
            },
            DaylioCustomMood {
                id: 4,
                custom_name: "Mood".to_owned(),
                mood_group_id: 4,
                mood_group_order: 0,
                icon_id: 1,
                predefined_name_id: -1,
                state: 0,
                created_at: 14582,
            },
            // this one is unique
            DaylioCustomMood {
                id: 5,
                custom_name: "Unique".to_owned(),
                icon_id: 1,
                predefined_name_id: -1,
                state: 0,
                mood_group_id: 3,
                mood_group_order: 4,
                created_at: 2,
            },
        ];

        let duplicate_tag = DaylioTag {
            id: 2,
            name: "Duplicate name".to_owned(),
            created_at: 1278,
            icon: 2,
            order: 1,
            state: 1,
            id_tag_group: 1,
        };
        let unique_tag = DaylioTag {
            id: 3,
            name: "Unique".to_owned(),
            created_at: 1278,
            icon: 2,
            order: 1,
            state: 1,
            id_tag_group: 1,
        };

        // next 2: same text, but with line breaks
        // title is separated in one, text in the other
        // date is slightly different
        // same mood, same tags
        let original_entry = DaylioDayEntry {
            id: 2,
            minute: 20,
            hour: 11, // <-- different
            day: 5,
            month: 4,
            year: 2021,
            datetime: 1617621600000, // <-- different
            time_zone_offset: 1,     // <-- different
            mood: 1,
            note: "This is a note with a line break\n".to_owned(),
            note_title: "Note title".to_owned(),
            tags: vec![],
            assets: vec![],
        };

        let pdf_entry = DaylioDayEntry {
            id: 1,
            minute: 20,
            hour: 10,
            day: 5,
            month: 4,
            year: 2021,
            datetime: 1617618000000,
            time_zone_offset: 0,
            mood: 2,
            note: "Note titleThis is a note with a line break\n".to_owned(),
            note_title: "".to_owned(),
            tags: vec![],
            assets: vec![],
        };

        let original_entry2 = DaylioDayEntry {
            id: 1,
            minute: 1,
            hour: 11,
            day: 23,
            month: 6,
            year: 2022,
            datetime: 1658566889780,
            time_zone_offset: 7200000,
            mood: 1,
            note: "Je viens de discuter avec mamie. Je le savais déjà, mais elle m'a répété qu'elle avait d'être restée".to_owned(),
            note_title: String::new(),
            tags: vec![],
            assets: vec![],
        };

        let pdf_entry2 = DaylioDayEntry {
            id: 12,
            minute: 1,
            hour: 11,
            day: 23,
            month: 6,
            year: 2022,
            datetime: 1658574060000,
            time_zone_offset: 0,
            mood: 2,
            note: "Je\nviens de discuter avec mamie. -Je le savais déjà, mais elle m'a répété qu'elle avait d'être\nrestée ...\n".to_owned(),
            note_title: String::new(),
            tags: vec![],
            assets: vec![],
        };

        let original_daylio = Daylio {
            tags: vec![duplicate_tag.clone(), unique_tag.clone()],
            day_entries: vec![original_entry.clone(), original_entry2.clone()],
            ..Default::default()
        };

        let mut pdf_daylio = Daylio {
            custom_moods,
            tags: vec![duplicate_tag],
            day_entries: vec![pdf_entry, pdf_entry2],
            ..Default::default()
        };
        pdf_daylio.sanitize(); // add default moods

        // remove duplicates
        let merged = merge(Diary::from(original_daylio), Diary::from(pdf_daylio)).unwrap();

        // check that there are no duplicates
        assert_eq!(merged.moods.len(), 5);
        assert_eq!(merged.tags.len(), 2);

        // the entries from left are preserved
        assert_eq!(merged.day_entries.len(), 2);
        let expected_entry1 = DayEntry {
            date: DateTime::from_timestamp_millis(original_entry.datetime)
                .unwrap()
                .naive_utc(),
            mood: Some(Mood::new("super")),
            tags: HashSet::new(),
            note: "Note title\n\nThis is a note with a line break\n".to_owned(),
        };
        let expected_entry2 = DayEntry {
            date: DateTime::from_timestamp_millis(original_entry2.datetime)
                .unwrap()
                .naive_utc(),
            mood: Some(Mood::new("super")),
            tags: HashSet::new(),
            note: original_entry2.note,
        };

        assert_eq!(merged.day_entries[0], expected_entry1);
        assert_eq!(merged.day_entries[1], expected_entry2);
    }

    fn base_input() -> Daylio {
        Daylio {
            version: 15,
            custom_moods: vec![
                DaylioCustomMood {
                    id: 1,
                    custom_name: "".to_owned(),
                    mood_group_id: 1,
                    mood_group_order: 0,
                    icon_id: 1,
                    predefined_name_id: 1,
                    state: 0,
                    created_at: 1651129353725,
                },
                DaylioCustomMood {
                    id: 2,
                    custom_name: "".to_owned(),
                    mood_group_id: 2,
                    mood_group_order: 0,
                    icon_id: 2,
                    predefined_name_id: 2,
                    state: 0,
                    created_at: 1651129353725,
                },
                DaylioCustomMood {
                    id: 3,
                    custom_name: "".to_owned(),
                    mood_group_id: 3,
                    mood_group_order: 0,
                    icon_id: 3,
                    predefined_name_id: 3,
                    state: 0,
                    created_at: 1651129353725,
                },
                DaylioCustomMood {
                    id: 4,
                    custom_name: "".to_owned(),
                    mood_group_id: 4,
                    mood_group_order: 0,
                    icon_id: 4,
                    predefined_name_id: 4,
                    state: 0,
                    created_at: 1651129353725,
                },
                DaylioCustomMood {
                    id: 5,
                    custom_name: "".to_owned(),
                    mood_group_id: 5,
                    mood_group_order: 0,
                    icon_id: 5,
                    predefined_name_id: 5,
                    state: 0,
                    created_at: 1651129353725,
                },
            ],
            ..Daylio::default()
        }
    }

    fn input1() -> Daylio {
        let mut input = base_input();
        input.custom_moods.insert(
            1,
            DaylioCustomMood {
                id: 6,
                custom_name: "custom".to_owned(),
                mood_group_id: 1,
                mood_group_order: 1,
                icon_id: 28,
                predefined_name_id: -1,
                state: 0,
                created_at: 1651129353725,
            },
        );

        input.tags = vec![
            DaylioTag {
                id: 24,
                name: "tag1".to_owned(),
                created_at: 1651129353707,
                icon: 41,
                order: 1,
                state: 0,
                id_tag_group: 1,
            },
            DaylioTag {
                id: 28,
                name: "tag2".to_owned(),
                created_at: 1651129353711,
                icon: 91,
                order: 5,
                state: 0,
                id_tag_group: 2,
            },
        ];

        input.day_entries = vec![
            DaylioDayEntry {
                id: 1,
                minute: 0,
                hour: 1,
                day: 3,
                month: 7,
                year: 2022,
                datetime: 1659481200000,
                time_zone_offset: 7200000,
                mood: 6,
                note: "".to_owned(),
                note_title: "".to_owned(),
                tags: vec![24],
                assets: vec![],
            },
            DaylioDayEntry {
                id: 2,
                minute: 0,
                hour: 20,
                day: 2,
                month: 7,
                year: 2022,
                datetime: 1659463200000,
                time_zone_offset: 7200000,
                mood: 1,
                note: "1".to_owned(),
                note_title: "".to_owned(),
                tags: vec![28],
                assets: vec![],
            },
            DaylioDayEntry {
                id: 3,
                minute: 45,
                hour: 22,
                day: 1,
                month: 7,
                year: 2022,
                datetime: 1659386700000,
                time_zone_offset: 7200000,
                mood: 1,
                note: "2".to_owned(),
                note_title: "".to_owned(),
                tags: vec![24, 28],
                assets: vec![],
            },
        ];
        input.metadata.number_of_entries = 3;

        input
    }

    #[test]
    fn merge_with_empty_do_not_change() -> Result<()> {
        let input1 = input1();
        let input2 = Daylio::default();

        let mut expected = input1.clone();

        println!("input1: {:#?}", input1.custom_moods);

        expected.tags = vec![
            DaylioTag {
                id: 1,
                name: "tag1".to_owned(),
                created_at: 1651129353707,
                icon: 41,
                order: 1,
                state: 0,
                id_tag_group: 1,
            },
            DaylioTag {
                id: 2,
                name: "tag2".to_owned(),
                created_at: 1651129353711,
                icon: 91,
                order: 2,
                state: 0,
                id_tag_group: 2,
            },
        ];

        for entry in expected.day_entries.iter_mut() {
            entry.tags = entry
                .tags
                .iter()
                .map(|id| match id {
                    24 => 1,
                    28 => 2,
                    _ => panic!("Unexpected tag id"),
                })
                .collect();
        }

        let merged = merge(Diary::from(input1), Diary::from(input2))?;

        assert_eq!(merged, Diary::from(expected));

        Ok(())
    }

    #[test]
    #[ignore]
    fn real_world_data() -> Result<()> {
        let input1 = load_daylio_backup("tests/data/old.daylio".as_ref())?;
        let input2 = load_daylio_backup("tests/data/new.daylio".as_ref())?;

        let expected = load_daylio_backup("tests/data/merged.daylio".as_ref())?;

        let merged = merge(Diary::from(input1), Diary::from(input2))?;

        assert_eq!(merged, Diary::from(expected));

        Ok(())
    }
}
