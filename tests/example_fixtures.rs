use cageplate::{
    cage_to_plate,
    config::AppConfig,
    plate_to_cage,
    tsv::{parse_tsv, write_tsv},
};

#[test]
fn plate_to_cage_matches_supplied_fixture() {
    let input = parse_tsv(include_str!("../example/plate2cage.input.txt")).unwrap();
    let config: AppConfig =
        serde_json::from_str(include_str!("fixtures/plate-to-cage.example.json")).unwrap();
    let output = plate_to_cage::convert(&input, &config.plate_to_cage).unwrap();
    // Read as raw records: the first hierarchy row intentionally has blank cells.
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_reader(include_bytes!("../example/plate2cage.output.txt").as_slice());
    let records: Vec<Vec<String>> = reader
        .records()
        .map(|r| r.unwrap().iter().map(String::from).collect())
        .collect();
    assert_eq!(output.headers, records[..2]);
    assert_eq!(output.rows, records[2..]);
    assert_eq!(
        normalized(&write_tsv(&output).unwrap()),
        normalized(include_str!("../example/plate2cage.output.txt"))
    );
}

fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}

#[test]
fn cage_to_plate_matches_supplied_fixture() {
    let input = parse_tsv(include_str!("../example/cage2plate.input.txt")).unwrap();
    let config: AppConfig =
        serde_json::from_str(include_str!("fixtures/cage-to-plate.example.json")).unwrap();
    let output = cage_to_plate::convert(&input, &config.cage_to_plate).unwrap();
    let expected = parse_tsv(include_str!("../example/cage2plate.output.txt")).unwrap();
    assert_eq!(output.headers, [expected.headers]);
    assert_eq!(output.rows, expected.rows);
    assert_eq!(
        normalized(&write_tsv(&output).unwrap()),
        normalized(include_str!("../example/cage2plate.output.txt"))
    );
}
