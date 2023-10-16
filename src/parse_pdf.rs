//! This module parses the PDF file without any interpretation
#![allow(dead_code)]

use std::fmt::{Debug, Display};
use std::mem;
use std::path::Path;

use chrono::NaiveDate;
use color_eyre::eyre::{ContextCompat, WrapErr};
use color_eyre::Result;
use nom::branch::alt;
use nom::bytes::complete::{take_till, take_until};
use nom::character::complete::{digit1, line_ending, multispace0, one_of, space0};
use nom::combinator::{eof, map, map_res};
use nom::multi::{count, many_till};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::{Finish, Parser};
use pdftotext::pdftotext_layout;

type IResult<I, O> = nom::IResult<I, O, nom::error::VerboseError<I>>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StatLine {
    pub(crate) name: String,
    pub(crate) count: u32,
}

impl StatLine {
    fn new(name: String, count: u32) -> Self {
        Self { name, count }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DayEntry {
    pub(crate) date: NaiveDate,
    pub(crate) day_hour: String,
    pub(crate) mood: String,
    pub(crate) note: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedPdf {
    pub(crate) stats: Vec<StatLine>,
    pub(crate) day_entries: Vec<DayEntry>,
}

fn extract_txt(pdf: &Path) -> Result<String> {
    let path = pdf.to_str().wrap_err("Invalid path")?;
    let txt = pdftotext_layout(path)?;

    Ok(txt.join(""))
}

fn read_line(input: &str) -> IResult<&str, &str> {
    map(
        terminated(take_till(|c| c == '\n'), line_ending),
        |line: &str| line.trim(),
    )(input)
}

fn parse_header(input: &str) -> IResult<&str, Vec<&str>> {
    map(many_till(read_line, count(line_ending, 3)), |(lines, _)| {
        lines
    })(input)
}

fn parse_stat_line(input: &str) -> IResult<&str, StatLine> {
    map(
        preceded(
            multispace0,
            tuple((
                terminated(take_until("  "), multispace0),
                map_res(terminated(digit1, one_of("×x")), str::parse::<u32>),
            )),
        ),
        |(name, count)| StatLine::new(name.to_string(), count),
    )(input)
}

fn parse_stat_lines(input: &str) -> IResult<&str, Vec<StatLine>> {
    map(
        many_till(parse_stat_line, count(line_ending, 4)),
        |(tags, _)| tags,
    )(input)
}

/// differences between english and french:
/// - month names
/// - "month day, year" becomes "day month year" in french
/// - 24 hour clock
fn convert_french_date(date: &str) -> Option<String> {
    let date = date.to_lowercase();
    let month_dict = [
        ("janvier", "january"),
        ("février", "february"),
        ("mars", "march"),
        ("avril", "april"),
        ("mai", "may"),
        ("juin", "june"),
        ("juillet", "july"),
        ("août", "august"),
        ("septembre", "september"),
        ("octobre", "october"),
        ("novembre", "november"),
        ("décembre", "december"),
    ];

    if !month_dict.iter().any(|(french, _)| date.contains(french)) {
        return Some(date); // not a french date
    }

    let mut date_parts = date.split_whitespace();
    let day = date_parts.next()?;

    let en_month = date_parts.next()?;
    let month = month_dict
        .iter()
        .find(|(french, _)| *french == en_month)
        .map(|(_, english)| english)?;

    let year = date_parts.next()?;

    Some(format!("{month} {day}, {year}"))
}

fn convert_language_date(date: &str) -> Option<String> {
    convert_french_date(date)
}

/// Date looks like August 2, 2022
fn string_to_date(date: &str) -> Result<NaiveDate> {
    let date = convert_language_date(date).wrap_err("Invalid date")?;
    NaiveDate::parse_from_str(&date, "%B %d, %Y").wrap_err(format!("Invalid date: {date}"))
}

fn parse_date(input: &str) -> IResult<&str, NaiveDate> {
    map_res(take_until("  "), string_to_date)(input)
}

/// Example: ALL CAPS MOOD\n
fn parse_mood(input: &str) -> IResult<&str, &str> {
    read_line(input)
}

/// Example: Sunday 8 53 PM\n
fn parse_day_hour(input: &str) -> IResult<&str, &str> {
    read_line(input)
}

/// There may be a title, but there's no way for us to know if there is one
/// So we count it as part of the body
/// Pseudo-regex
/// ```raw
/// (\n{0, 1}([^\n]{1, n}, \n){1, n}\n{2, 3})
/// ```
fn parse_note_body(input: &str) -> IResult<&str, (Vec<&str>, Option<NaiveDate>)> {
    // The body is a series of lines, separated by line endings
    let body = alt((
        parse_page_number.map(|_| None), // page numbers can be intertwined with the note
        read_line.map(Some),
    ));

    let date_or_eof = alt((parse_date.map(Some), eof.map(|_| None)));

    let body = many_till(body, date_or_eof).map(|(lines, date)| {
        let no_empty_lines = lines.into_iter().flatten().filter(|l| !l.is_empty());

        (no_empty_lines.collect(), date)
    });

    preceded(multispace0, body)(input)
}

/// A day entry looks like this:
/// ```raw
/// May 22, 2022              RAD
/// Sunday 8 53 PM
///                              Tag 2 NWR    Tag 4 HBK   Tag 5 IGN     Tag 10 OKU     Tag 23 CLN
///                              Tag 14 NEU   Tag 21 NUD    Tag 22 ITV
///                     Optional Note title 35 XLA
///                     Note 35 AHM
/// ```
/// But it may also have no body
/// ```raw
/// May 22, 2022              RAD
/// Sunday 8 53 PM
/// ```
/// Seen as a pseudo regex:
/// ```raw
/// date {2, n}mood\nday hour\n(\n\n|\n{0, 1}([^\n]{1, n}, \n){1, n}\n{2, 3})
/// ```
/// body can also be ended by `\nEOF`
fn parse_day_entries(input: &str) -> IResult<&str, Vec<DayEntry>> {
    // So, we are in some kind of weird situation here.
    // We use the date as a separator, as it is the only thing that is guaranteed to be there.
    // But the date is the first thing we parse, so we're gonna be off by one.

    let (input, mut prev_date) = map(parse_date, Some)(input)?;

    let parse_day = map(
        tuple((parse_mood, parse_day_hour, parse_note_body)),
        |(mood, day_hour, (note, next_date))| {
            prev_date?; // if there's no date, we're at the end of the file

            let note = note.into_iter().map(ToOwned::to_owned).collect();

            Some(DayEntry {
                date: mem::replace(&mut prev_date, next_date).unwrap(),
                mood: mood.to_owned(),
                day_hour: day_hour.to_owned(),
                note,
            })
        },
    );

    let res = map(many_till(parse_day, eof), |(days, _)| days)(input);
    res.map(|(input, days)| (input, days.into_iter().flatten().collect()))
}

fn parse_page_number(input: &str) -> IResult<&str, &str> {
    delimited(space0, digit1, line_ending)(input)
}

#[derive(Debug, Clone)]
struct ParsePdfError {
    json: String,
}

impl Display for ParsePdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse PDF:\n{}", self.json)
    }
}

impl std::error::Error for ParsePdfError {}

pub(crate) fn parse_pdf(path: &Path) -> Result<ParsedPdf> {
    let text = extract_txt(path)?;
    let input = text.as_str();

    let first_page = preceded(parse_header, parse_stat_lines);

    let mut parser = tuple((first_page, parse_day_entries));

    parser(input)
        .finish()
        .map(|(_, (stats, day_entries))| ParsedPdf { stats, day_entries })
        .map_err(|e| {
            ParsePdfError {
                json: nom::error::convert_error(input, e),
            }
            .into()
        })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::Read;

    use similar_asserts::assert_eq;

    use super::*;

    const SMALL_PDF_PATH_ENGLISH: &str = "tests/data/official/english.pdf";

    #[test]
    fn test_parse_small_pdf() -> Result<()> {
        // syntax:

        let actual = parse_pdf(SMALL_PDF_PATH_ENGLISH.as_ref())?;

        let expected = ParsedPdf {
            stats: vec![
                StatLine {
                    name: "rad".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "manger sain".to_owned(),
                    count: 2,
                },
                StatLine {
                    name: "exercice".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "meh".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "ménage".to_owned(),
                    count: 2,
                },
                StatLine {
                    name: "sport".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "null".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "famille".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "films".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "awful".to_owned(),
                    count: 2,
                },
                StatLine {
                    name: "rendez vous".to_owned(),
                    count: 1,
                },
                StatLine {
                    name: "shopping".to_owned(),
                    count: 1,
                },
            ],
            day_entries: vec![
                DayEntry {
                    date: string_to_date("January 24, 2023").unwrap(),
                    day_hour: "Tuesday 11 36 AM".to_owned(),
                    mood: "AWFUL".to_owned(),
                    note: vec![],
                }, DayEntry {
                    date: string_to_date("January 24, 2023").unwrap(),
                    day_hour: "Tuesday 9 59 AM".to_owned(),
                    mood: "RAD".to_owned(),
                    note: vec![
                        "famille       rendez vous        exercice         sport       ménage".to_owned(),
                        "Note title".to_owned(),
                        "Note body".to_owned(),
                    ],
                },
                DayEntry {
                    date: string_to_date("January 11, 2023").unwrap(),
                    day_hour: "Wednesday 10 20 PM".to_owned(),
                    mood: "MEH".to_owned(),
                    note: vec![
                        "manger sain".to_owned(),
                        "Hey, here's a note with".to_owned(),
                        "Linebreaks!".to_owned(),
                        "Because I love breaking parsers".to_owned(),
                    ],
                },
                DayEntry {
                    date: string_to_date("January 4, 2023").unwrap(),
                    day_hour: "Wednesday 8 00 PM".to_owned(),
                    mood: "AWFUL".to_owned(),
                    note: vec![
                        "manger sain        films       ménage          shopping".to_owned(),
                    ],
                },
                DayEntry {
                    date: string_to_date("May 16, 2015").unwrap(),
                    day_hour: "Saturday 8 00 PM".to_string(),
                    mood: "NULL".to_string(),
                    note: vec!["No tag".to_owned(),
                               "This is an old note. It has no title, but its body is really longThis is an old note. It has no title, but".to_owned(),
                               "its body is really long".to_owned(),
                               "This is an old note. It has no title, but its body is really long".to_owned(),
                               "This is an old note. It has no title, but its body is really longThis is an old note. It has no title, but".to_owned(),
                               "its body is really long".to_owned(),
                               "This is an old note. It has no title, but its body is really longThis is an old note. It has no title, but".to_owned(),
                               "its body is really long".to_owned(),
                               "This is an old note. It has no title, but its body is really long".to_owned(),
                               "This is an old note. It has no title, but its body is really longThis is an old note. It has no title, but".to_owned(),
                               "its body is really long".to_owned(),
                    ],
                },
            ],
        };

        assert_eq!(actual, expected);

        Ok(())
    }

    // All the tests below are for the large PDF

    const TEST_PDF: &str = "tests/data/new.pdf";
    const TEST_PDF_TXT: &str = "tests/data/new_extracted.txt";

    fn get_txt() -> String {
        let mut file = std::fs::File::open(TEST_PDF_TXT).unwrap();
        let mut res = String::new();
        file.read_to_string(&mut res).unwrap();
        res
    }

    #[test]
    fn extract_txt_test() {
        let txt = extract_txt(Path::new(TEST_PDF)).unwrap();
        let expected = get_txt();
        assert_eq!(txt, expected);
    }

    #[test]
    fn test_parse_header() {
        let txt = get_txt();
        let input = txt.as_str();

        let parsed = parse_header(input).unwrap();

        let expected_in = &input[111..];
        let expected_parsed = vec![
            "Daylio Export                                                           1".to_owned(),
            "April 27, 2022 - January 23, 2023".to_owned(),
        ];
        assert_eq!(parsed.0, expected_in);
        assert_eq!(parsed.1, expected_parsed);
    }

    pub(crate) fn expected_parsed_tags() -> Vec<StatLine> {
        /*
            rad                     15×        Tag 21 NUD   9×   Tag 8 WNA    2×
            Mood 0 KWY               5×        Tag 11 XRB   8×   Tag 14 NEU   2×
            good                    20×        Tag 6 AUG    6×   Tag 9 MAS    1×
            Mood 1 QBL              13×        Tag 10 OKU   6×   Tag 16 QUG   1×
            meh                      1×        Tag 23 CLN   5×   Tag 22 ITV   1×
            Mood 2 VUP               8×        Tag 2 NWR    4×   Tag 24 KVI   1×
            bad                      2×        Tag 12 LRD   3×   Tag 25 CGQ   1×
            Tag 5 IGN               14×        Tag 0 AHY    2×   Tag 33 IQP   1×
            Tag 4 HBK               10×
        */
        vec![
            StatLine::new("rad".to_owned(), 15),
            StatLine::new("Tag 21 NUD".to_owned(), 9),
            StatLine::new("Tag 8 WNA".to_owned(), 2),
            StatLine::new("Mood 0 KWY".to_owned(), 5),
            StatLine::new("Tag 11 XRB".to_owned(), 8),
            StatLine::new("Tag 14 NEU".to_owned(), 2),
            StatLine::new("good".to_owned(), 20),
            StatLine::new("Tag 6 AUG".to_owned(), 6),
            StatLine::new("Tag 9 MAS".to_owned(), 1),
            StatLine::new("Mood 1 QBL".to_owned(), 13),
            StatLine::new("Tag 10 OKU".to_owned(), 6),
            StatLine::new("Tag 16 QUG".to_owned(), 1),
            StatLine::new("meh".to_owned(), 1),
            StatLine::new("Tag 23 CLN".to_owned(), 5),
            StatLine::new("Tag 22 ITV".to_owned(), 1),
            StatLine::new("Mood 2 VUP".to_owned(), 8),
            StatLine::new("Tag 2 NWR".to_owned(), 4),
            StatLine::new("Tag 24 KVI".to_owned(), 1),
            StatLine::new("bad".to_owned(), 2),
            StatLine::new("Tag 12 LRD".to_owned(), 3),
            StatLine::new("Tag 25 CGQ".to_owned(), 1),
            StatLine::new("Tag 5 IGN".to_owned(), 14),
            StatLine::new("Tag 0 AHY".to_owned(), 2),
            StatLine::new("Tag 33 IQP".to_owned(), 1),
            StatLine::new("Tag 4 HBK".to_owned(), 10),
        ]
    }

    #[test]
    fn test_parse_stats() {
        let txt = get_txt();
        let input = parse_header(txt.as_str()).unwrap().0; // skip header

        let parsed = parse_stat_lines(input).unwrap();

        let expected_in = &input[661..];
        let expected_parsed = expected_parsed_tags();

        assert_eq!(parsed.0, expected_in);
        assert_eq!(parsed.1, expected_parsed);
    }

    #[test]
    fn test_parse_pdf() {
        let parsed = parse_pdf(Path::new(TEST_PDF)).unwrap();
        let expected_tags = expected_parsed_tags();

        let expected_entries = vec![
            DayEntry {
                date: string_to_date("August 2, 2022").unwrap(),
                day_hour: "Tuesday 11 00 PM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 0 LKH".to_owned(), "Note 0 LHF".to_owned()],
            },
            DayEntry {
                date: string_to_date("August 2, 2022").unwrap(),
                day_hour: "Tuesday 6 00 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 1 OAK".to_owned(), "Note 1 QJO".to_owned()],
            },
            DayEntry {
                date: string_to_date("August 1, 2022").unwrap(),
                day_hour: "Monday 8 45 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 2 FFU".to_owned(), "Note 2 JBQ".to_owned()],
            },
            DayEntry {
                date: string_to_date("August 1, 2022").unwrap(),
                day_hour: "Monday 10 30 AM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 3 MKL".to_owned(), "Note 3 VPH".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 31, 2022").unwrap(),
                day_hour: "Sunday 4 00 PM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 4 BTD".to_owned(), "Note 4 UDK".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 30, 2022").unwrap(),
                day_hour: "Saturday 9 00 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 5 VXG".to_owned(), "Note 5 AOT".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 29, 2022").unwrap(),
                day_hour: "Friday 8 00 AM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 6 JIG".to_owned(), "Note 6 GVX".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 25, 2022").unwrap(),
                day_hour: "Monday 10 01 AM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 7 IFI".to_owned(), "Note 7 ABH".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 23, 2022").unwrap(),
                day_hour: "Saturday 10 58 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 8 AGV".to_owned(), "Note 8 UGW".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 23, 2022").unwrap(),
                day_hour: "Saturday 9 01 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 9 VGL".to_owned(), "Note 9 XMI".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 23, 2022").unwrap(),
                day_hour: "Saturday 7 44 AM".to_owned(),
                mood: "MEH".to_owned(),
                note: vec!["Note title 10 YIG".to_owned(), "Note 10 ADT".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 23, 2022").unwrap(),
                day_hour: "Saturday 7 26 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 11 FSE".to_owned(), "Note 11 GUP".to_owned()],
            },
            DayEntry {
                date: string_to_date("July 1, 2022").unwrap(),
                day_hour: "Friday 9 19 PM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 12 LGC".to_owned(), "Note 12 XKN".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 30, 2022").unwrap(),
                day_hour: "Thursday 6 39 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 13 AKM".to_owned(), "Note 13 YJP".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 26, 2022").unwrap(),
                day_hour: "Sunday 5 00 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 14 CGY".to_owned(), "Note 14 XHV".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 23, 2022").unwrap(),
                day_hour: "Thursday 12 52 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 15 IQK".to_owned(), "Note 15 JJD".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 23, 2022").unwrap(),
                day_hour: "Thursday 12 05 PM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 16 RDS".to_owned(), "Note 16 TYC".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 23, 2022").unwrap(),
                day_hour: "Thursday 8 04 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 17 MCA".to_owned(), "Note 17 FGP".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 22, 2022").unwrap(),
                day_hour: "Wednesday 6 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 18 BFC".to_owned(), "Note 18 VLP".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 20, 2022").unwrap(),
                day_hour: "Monday 9 00 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 19 OVK".to_owned(), "Note 19 BIB".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 19, 2022").unwrap(),
                day_hour: "Sunday 9 29 PM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 20 IJG".to_owned(), "Note 20 JWW".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 18, 2022").unwrap(),
                day_hour: "Saturday 9 29 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 21 YYM".to_owned(), "Note 21 LGX".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 13, 2022").unwrap(),
                day_hour: "Monday 9 25 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 22 DDS".to_owned(), "Note 22 PDV".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 11, 2022").unwrap(),
                day_hour: "Saturday 10 00 AM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 23 HWK".to_owned(), "Note 23 IXE".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 9, 2022").unwrap(),
                day_hour: "Thursday 9 14 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 24 EXK".to_owned(), "Note 24 NHO".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 9, 2022").unwrap(),
                day_hour: "Thursday 10 21 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 25 HVQ".to_owned(), "Note 25 KLA".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 6, 2022").unwrap(),
                day_hour: "Monday 8 50 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 26 ONQ".to_owned(), "Note 26 DCC".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 4, 2022").unwrap(),
                day_hour: "Saturday 9 50 PM".to_owned(),
                mood: "MOOD 0 KWY".to_owned(),
                note: vec!["Note title 27 PBF".to_owned(), "Note 27 BGL".to_owned()],
            },
            DayEntry {
                date: string_to_date("June 3, 2022").unwrap(),
                day_hour: "Friday 10 24 AM".to_owned(),
                mood: "MOOD 0 KWY".to_owned(),
                note: vec!["Note title 28 FGA".to_owned(), "Note 28 AEQ".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 29, 2022").unwrap(),
                day_hour: "Sunday 8 42 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 29 AIU".to_owned(), "Note 29 GVL".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 28, 2022").unwrap(),
                day_hour: "Saturday 6 00 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 30 RRM".to_owned(), "Note 30 QVS".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 27, 2022").unwrap(),
                day_hour: "Friday 8 42 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 31 LPS".to_owned(), "Note 31 HKU".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 26, 2022").unwrap(),
                day_hour: "Thursday 8 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 32 MGE".to_owned(), "Note 32 PRG".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 25, 2022").unwrap(),
                day_hour: "Wednesday 4 55 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 33 AMR".to_owned(), "Note 33 MYX".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 24, 2022").unwrap(),
                day_hour: "Tuesday 8 44 PM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec!["Note title 34 YRH".to_owned(), "Note 34 SXS".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 22, 2022").unwrap(),
                day_hour: "Sunday 8 53 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec![
                    "Tag 2 NWR    Tag 4 HBK   Tag 5 IGN     Tag 10 OKU     Tag 23 CLN".to_owned(),
                    "Note title 35 XLA".to_owned(),
                    "Note 35 AHM".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 20, 2022").unwrap(),
                day_hour: "Friday 8 15 PM".to_owned(),
                mood: "MOOD 0 KWY".to_owned(),
                note: vec![
                    "Tag 5 IGN    Tag 6 AUG   Tag 21 NUD     Tag 23 CLN".to_owned(),
                    "Note title 36 GYK".to_owned(),
                    "Note 36 AFX".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 20, 2022").unwrap(),
                day_hour: "Friday 5 11 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 37 SHL".to_owned(), "Note 37 YKU".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 15, 2022").unwrap(),
                day_hour: "Sunday 9 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 5 IGN   Tag 6 AUG     Tag 11 XRB    Tag 21 NUD".to_owned(),
                    "Tag 23 CLN".to_owned(),
                    "Note title 38 NBR".to_owned(),
                    "Note 38 HPJ".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 14, 2022").unwrap(),
                day_hour: "Saturday 1 50 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 8 WNA    Tag 12 LRD    Tag 33 IQP".to_owned(),
                    "Note title 39 UKI".to_owned(),
                    "Note 39 KFO".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 13, 2022").unwrap(),
                day_hour: "Friday 6 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 0 AHY    Tag 5 IGN    Tag 6 AUG     Tag 11 XRB".to_owned(),
                    "Note title 40 TJJ".to_owned(),
                    "Note 40 DBV".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 12, 2022").unwrap(),
                day_hour: "Thursday 7 04 AM".to_owned(),
                mood: "BAD".to_owned(),
                note: vec!["Note title 41 EBK".to_owned(), "Note 41 HVI".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 11, 2022").unwrap(),
                day_hour: "Wednesday 11 17 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec!["Note title 42 OLY".to_owned(), "Note 42 FQU".to_owned()],
            },
            DayEntry {
                date: string_to_date("May 11, 2022").unwrap(),
                day_hour: "Wednesday 9 39 AM".to_owned(),
                mood: "BAD".to_owned(),
                note: vec![
                    "Tag 5 IGN    Tag 6 AUG    Tag 10 OKU".to_owned(),
                    "Note title 43 VXJ".to_owned(),
                    "Note 43 MBW".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 10, 2022").unwrap(),
                day_hour: "Tuesday 9 57 AM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec![
                    "Tag 5 IGN    Tag 9 MAS    Tag 10 OKU".to_owned(),
                    "Note title 44 DPR".to_owned(),
                    "Note 44 BIV".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 9, 2022").unwrap(),
                day_hour: "Monday 8 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 5 IGN    Tag 6 AUG    Tag 12 LRD    Tag 21 NUD".to_owned(),
                    "Note title 45 LWT".to_owned(),
                    "Note 45 OUF".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 8, 2022").unwrap(),
                day_hour: "Sunday 8 27 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec![
                    "Tag 2 NWR    Tag 4 HBK    Tag 5 IGN     Tag 6 AUG    Tag 10 OKU".to_owned(),
                    "Tag 14 NEU   Tag 21 NUD    Tag 22 ITV".to_owned(),
                    "Note title 46 EAJ".to_owned(),
                    "Note 46 FWU".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 7, 2022").unwrap(),
                day_hour: "Saturday 7 00 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec![
                    "Tag 2 NWR    Tag 4 HBK    Tag 5 IGN     Tag 10 OKU".to_owned(),
                    "Note title 47 NYG".to_owned(),
                    "Note 47 AND".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 6, 2022").unwrap(),
                day_hour: "Friday 5 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 5 IGN    Tag 8 WNA     Tag 11 XRB".to_owned(),
                    "Note title 48 EEX".to_owned(),
                    "Note 48 NNJ".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 5, 2022").unwrap(),
                day_hour: "Thursday 8 37 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 5 IGN     Tag 11 XRB   Tag 21 NUD   Tag 23 CLN".to_owned(),
                    "Note title 49 MFY".to_owned(),
                    "Note 49 AFH".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 4, 2022").unwrap(),
                day_hour: "Wednesday 8 45 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 5 IGN     Tag 21 NUD   Tag 25 CGQ".to_owned(),
                    "Note title 50 THD".to_owned(),
                    "Note 50 USB".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 3, 2022").unwrap(),
                day_hour: "Tuesday 6 31 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 5 IGN     Tag 11 XRB   Tag 21 NUD".to_owned(),
                    "Note title 51 OXM".to_owned(),
                    "Note 51 DMN".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 2, 2022").unwrap(),
                day_hour: "Monday 8 00 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 21 NUD".to_owned(),
                    "Note title 52 MCT".to_owned(),
                    "Note 52 VUF".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 2, 2022").unwrap(),
                day_hour: "Monday 5 12 PM".to_owned(),
                mood: "MOOD 2 VUP".to_owned(),
                note: vec![
                    "Tag 4 HBK    Tag 12 LRD".to_owned(),
                    "Note title 53 JGL".to_owned(),
                    "Note 53 NTR".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("May 1, 2022").unwrap(),
                day_hour: "Sunday 3 19 PM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 2 NWR    Tag 4 HBK      Tag 5 IGN   Tag 11 XRB   Tag 14 NEU".to_owned(),
                    "Tag 16 QUG    Tag 23 CLN".to_owned(),
                    "Note title 54 JRN".to_owned(),
                    "Note 54 HOI".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("April 30, 2022").unwrap(),
                day_hour: "Saturday 1 30 PM".to_owned(),
                mood: "RAD".to_owned(),
                note: vec!["Note title 55 NWO".to_owned(), "Note 55 JGI".to_owned()],
            },
            DayEntry {
                date: string_to_date("April 30, 2022").unwrap(),
                day_hour: "Saturday 6 09 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec![
                    "Tag 0 AHY    Tag 10 OKU   Tag 21 NUD".to_owned(),
                    "Note title 56 WRY".to_owned(),
                    "Note 56 LOF".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("April 29, 2022").unwrap(),
                day_hour: "Friday 5 23 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 11 XRB".to_owned(),
                    "Note title 57 HHQ".to_owned(),
                    "Note 57 MHD".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("April 28, 2022").unwrap(),
                day_hour: "Thursday 5 01 PM".to_owned(),
                mood: "MOOD 0 KWY".to_owned(),
                note: vec!["Note title 58 AKY".to_owned(), "Note 58 CHG".to_owned()],
            },
            DayEntry {
                date: string_to_date("April 28, 2022").unwrap(),
                day_hour: "Thursday 8 24 AM".to_owned(),
                mood: "MOOD 0 KWY".to_owned(),
                note: vec![
                    "Tag 24 KVI".to_owned(),
                    "Note title 59 XNI".to_owned(),
                    "Note 59 XHR".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("April 28, 2022").unwrap(),
                day_hour: "Thursday 7 11 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 60 TEO".to_owned(), "Note 60 YQQ".to_owned()],
            },
            DayEntry {
                date: string_to_date("April 28, 2022").unwrap(),
                day_hour: "Thursday 7 02 AM".to_owned(),
                mood: "GOOD".to_owned(),
                note: vec![
                    "Tag 11 XRB".to_owned(),
                    "Note title 61 GTQ".to_owned(),
                    "Note 61 NJC".to_owned(),
                ],
            },
            DayEntry {
                date: string_to_date("April 27, 2022").unwrap(),
                day_hour: "Wednesday 1 00 PM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 62 OQP".to_owned(), "Note 62 BTP".to_owned()],
            },
            DayEntry {
                date: string_to_date("April 27, 2022").unwrap(),
                day_hour: "Wednesday 5 30 AM".to_owned(),
                mood: "MOOD 1 QBL".to_owned(),
                note: vec!["Note title 63 FSU".to_owned(), "Note 63 DWN".to_owned()],
            },
        ];

        let expected = ParsedPdf {
            stats: expected_tags,
            day_entries: expected_entries,
        };

        assert_eq!(parsed, expected);
    }
}
