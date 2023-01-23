//! This module parses the PDF file without any interpretation

use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::multi::{count, many_till};
use std::fmt::{Debug, Display};

use nom::character::complete::{digit1, line_ending, multispace0};
use nom::combinator::{map, map_res};
use nom::sequence::{preceded, terminated, tuple};
use nom::Finish;
use pdftotext::pdftotext_layout;
use std::path::Path;

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

pub(crate) struct ParsedPdf {
    stats: Vec<StatLine>,
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
                map_res(terminated(digit1, tag("×")), str::parse::<u32>),
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

    let parsed = preceded(parse_header, parse_stat_lines)(input);

    let result = parsed
        .finish()
        .map_err(|e| nom::error::convert_error(input, e))
        .map_err(|json| ParsePdfError { json })?;

    Ok(ParsedPdf { stats: result.1 })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Read;

    const TEST_PDF: &str = "tests/data/new.pdf";
    const TEST_PDF_TXT: &'static str = "tests/data/new_extracted.txt";

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
            "Daylio Export                                                           1",
            "April 27, 2022 - January 23, 2023",
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
            StatLine::new("rad".to_string(), 15),
            StatLine::new("Tag 21 NUD".to_string(), 9),
            StatLine::new("Tag 8 WNA".to_string(), 2),
            StatLine::new("Mood 0 KWY".to_string(), 5),
            StatLine::new("Tag 11 XRB".to_string(), 8),
            StatLine::new("Tag 14 NEU".to_string(), 2),
            StatLine::new("good".to_string(), 20),
            StatLine::new("Tag 6 AUG".to_string(), 6),
            StatLine::new("Tag 9 MAS".to_string(), 1),
            StatLine::new("Mood 1 QBL".to_string(), 13),
            StatLine::new("Tag 10 OKU".to_string(), 6),
            StatLine::new("Tag 16 QUG".to_string(), 1),
            StatLine::new("meh".to_string(), 1),
            StatLine::new("Tag 23 CLN".to_string(), 5),
            StatLine::new("Tag 22 ITV".to_string(), 1),
            StatLine::new("Mood 2 VUP".to_string(), 8),
            StatLine::new("Tag 2 NWR".to_string(), 4),
            StatLine::new("Tag 24 KVI".to_string(), 1),
            StatLine::new("bad".to_string(), 2),
            StatLine::new("Tag 12 LRD".to_string(), 3),
            StatLine::new("Tag 25 CGQ".to_string(), 1),
            StatLine::new("Tag 5 IGN".to_string(), 14),
            StatLine::new("Tag 0 AHY".to_string(), 2),
            StatLine::new("Tag 33 IQP".to_string(), 1),
            StatLine::new("Tag 4 HBK".to_string(), 10),
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
        let expected_parsed = expected_parsed_tags();
        assert_eq!(parsed.stats, expected_parsed);
    }
}
