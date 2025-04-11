#[cfg(test)]
mod tests {
    use color_eyre::Result;

    use daylio_tools::{CustomMood, DayEntry, Daylio, Tag, load_daylio_backup, merge};

    fn base_input() -> Daylio {
        Daylio {
            version: 15,
            custom_moods: vec![
                CustomMood {
                    id: 1,
                    custom_name: "".to_owned(),
                    mood_group_id: 1,
                    mood_group_order: 0,
                    icon_id: 1,
                    predefined_name_id: 1,
                    state: 0,
                    created_at: 1651129353725,
                },
                CustomMood {
                    id: 2,
                    custom_name: "".to_owned(),
                    mood_group_id: 2,
                    mood_group_order: 0,
                    icon_id: 2,
                    predefined_name_id: 2,
                    state: 0,
                    created_at: 1651129353725,
                },
                CustomMood {
                    id: 3,
                    custom_name: "".to_owned(),
                    mood_group_id: 3,
                    mood_group_order: 0,
                    icon_id: 3,
                    predefined_name_id: 3,
                    state: 0,
                    created_at: 1651129353725,
                },
                CustomMood {
                    id: 4,
                    custom_name: "".to_owned(),
                    mood_group_id: 4,
                    mood_group_order: 0,
                    icon_id: 4,
                    predefined_name_id: 4,
                    state: 0,
                    created_at: 1651129353725,
                },
                CustomMood {
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
            CustomMood {
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
            Tag {
                id: 24,
                name: "tag1".to_owned(),
                created_at: 1651129353707,
                icon: 41,
                order: 1,
                state: 0,
                id_tag_group: 1,
            },
            Tag {
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
            DayEntry {
                id: 1,
                minute: 0,
                hour: 1,
                day: 3,
                month: 7,
                year: 2022,
                datetime: 1659481200000,
                time_zone_offset: 7200000,
                mood: 8,
                note: "".to_owned(),
                note_title: "".to_owned(),
                tags: vec![24],
                assets: vec![],
            },
            DayEntry {
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
            DayEntry {
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

        expected.tags = vec![
            Tag {
                id: 1,
                name: "tag1".to_owned(),
                created_at: 1651129353707,
                icon: 41,
                order: 1,
                state: 0,
                id_tag_group: 1,
            },
            Tag {
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

        let merged = merge(input1, input2);

        assert_eq!(merged, expected);

        Ok(())
    }

    #[test]
    fn real_world_data() -> Result<()> {
        let input1 = load_daylio_backup("tests/data/old.daylio".as_ref())?;
        let input2 = load_daylio_backup("tests/data/new.daylio".as_ref())?;

        let expected = load_daylio_backup("tests/data/merged.daylio".as_ref())?;

        let merged = merge(input1, input2);

        assert_eq!(merged, expected);

        Ok(())
    }
}
