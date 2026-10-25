use cageplate::config::*;
use std::fs;

#[test]
fn absent_file_loads_without_creating_anything_and_save_is_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new-directory/defaults.json");
    let mut loaded = load_defaults(&path);
    assert!(loaded.warning.is_none());
    assert!(!path.parent().unwrap().exists());
    loaded.config.cage_to_plate.plate.start_mouse = 37;
    assert!(!path.exists());
    save_defaults(&path, &loaded.config).unwrap();
    let original = fs::read(&path).unwrap();
    loaded.config.cage_to_plate.plate.start_mouse = 42;
    assert_eq!(fs::read(&path).unwrap(), original);
    save_defaults(&path, &loaded.config).unwrap();
    assert_eq!(
        read_config(&path).unwrap().cage_to_plate.plate.start_mouse,
        42
    );
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn all_settings_roundtrip_without_animal_data() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    let mut config = AppConfig::default();
    config.cage_to_plate.ears = vec!["custom2".into(), "custom1".into()];
    config.cage_to_plate.layout = CageLayout::Inline;
    config.cage_to_plate.plate = PlateConfig {
        rows: 16,
        columns: 24,
        traversal: Traversal::RowWise,
        well_style: WellStyle::LetterNumber,
        start_row: 3,
        start_column: 5,
        start_plate: 7,
        start_mouse: 101,
    };
    config.plate_to_cage.groups = vec![GroupConfig {
        column: ColumnRef::Occurrence {
            header: "Gene".into(),
            occurrence: 1,
        },
        name: "Display".into(),
        mode: CategoryMode::All,
        labels: [(cageplate::model::Genotype::Wt, "WT".into())].into(),
        ..Default::default()
    }];
    config.plate_to_cage.mouse_label = "{ear}/{plate}/{well}".into();
    config.plate_to_cage.output.order = vec!["sex".into(), "cage".into()];
    config
        .plate_to_cage
        .output
        .labels
        .insert("unknown".into(), "Uncalled".into());
    save_defaults(&path, &config).unwrap();
    let loaded = load_defaults(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.config, config);
    let json: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(
        json.as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["cage_to_plate", "plate_to_cage", "schema_version"]
    );
    assert!(AppConfig::from_json("{\"pasted_input\":\"animal data\"}").is_err());
}

#[test]
fn corrupt_future_and_semantically_invalid_files_are_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    for text in [
        "not json",
        "{\"schema_version\":999}",
        "{\"cage_to_plate\":{\"plate\":{\"rows\":0}}}",
        "{\"plate_to_cage\":{\"tokens\":{\"hom\":\"+/+\"}}}",
        "{\"unknown_setting\":true}",
    ] {
        fs::write(&path, text).unwrap();
        let loaded = load_defaults(&path);
        assert!(loaded.warning.is_some(), "{text}");
        assert_eq!(loaded.config, AppConfig::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn invalid_save_cannot_overwrite_existing_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    save_defaults(&path, &AppConfig::default()).unwrap();
    let bytes = fs::read(&path).unwrap();
    let mut invalid = AppConfig::default();
    invalid.cage_to_plate.plate.rows = 0;
    assert!(save_defaults(&path, &invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(save_defaults(temp.path(), &AppConfig::default()).is_err());
    assert!(temp.path().is_dir());
}

#[test]
fn unknown_fields_in_duplicate_column_mapping_fail() {
    assert!(
        AppConfig::from_json(
            r#"{"cage_to_plate":{"cage":{"header":"Cage","occurrence":0,"typo":true}}}"#
        )
        .is_err()
    );
    assert!(AppConfig::from_json(r#"{"cage_to_plate":{"cage":""}}"#).is_err());
}
