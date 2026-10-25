use cageplate::{
    cage_to_plate,
    config::{AppConfig, ColumnRef},
    plate_to_cage,
    tsv::{parse_tsv, parse_tsv_with_header, write_tsv},
};

#[test]
fn cage_headerless_fixture_matches_headered_cells_and_tsv() {
    let source = include_str!("../example/cage2plate.input.txt");
    let headed = parse_tsv(source).unwrap();
    let table = parse_tsv_with_header(source.split_once('\n').unwrap().1, false).unwrap();
    let mut config = AppConfig::from_json(include_str!("fixtures/cage-to-plate.example.json"))
        .unwrap()
        .cage_to_plate;
    let expected = cage_to_plate::convert(&headed, &config).unwrap();
    config.has_header = false;
    for reference in [&mut config.cage, &mut config.quantity, &mut config.sex] {
        *reference = ColumnRef::Position {
            column: reference.resolve(&headed).unwrap() + 1,
        };
    }
    let actual = cage_to_plate::convert(&table, &config).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(write_tsv(&actual).unwrap(), write_tsv(&expected).unwrap());
}

#[test]
fn plate_fixture_without_header_names_is_rejected() {
    let source = include_str!("../example/plate2cage.input.txt");
    let table = parse_tsv_with_header(source.split_once('\n').unwrap().1, false).unwrap();
    let mut config = AppConfig::from_json(include_str!("fixtures/plate-to-cage.example.json"))
        .unwrap()
        .plate_to_cage;
    let error = plate_to_cage::convert(&table, &config)
        .unwrap_err()
        .to_string();
    assert!(error.contains("requires a header row"), "{error}");
    config.has_header = false;
    assert!(config.validate().is_err());
    assert!(plate_to_cage::convert(&table, &config).is_err());
}

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
