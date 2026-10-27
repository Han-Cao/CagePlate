use cageplate::config::*;
use std::fs;

#[test]
fn header_modes_and_positions_extend_existing_partial_schema() {
    let old = AppConfig::from_json(r#"{"cage_to_plate":{"cage":"Custom"}}"#).unwrap();
    assert!(old.cage_to_plate.has_header && old.plate_to_cage.has_header);
    let mut new = AppConfig::from_json(r#"{"cage_to_plate":{"has_header":false,"cage":{"column":1},"quantity":{"column":2},"sex":{"column":3}}}"#).unwrap();
    new.cage_to_plate.layout = CageLayout::SeparateColumn;
    assert!(AppConfig::from_json(r#"{"plate_to_cage":{"has_header":false}}"#).is_err());
    new.plate_to_cage.groups.push(GroupConfig {
        column: ColumnRef::Position { column: 7 },
        ..Default::default()
    });
    assert_eq!(
        AppConfig::from_json(&serde_json::to_string(&new).unwrap()).unwrap(),
        new
    );
    for mapping in [
        r#"{"column":0}"#,
        r#"{"column":4097}"#,
        r#"{"column":-1}"#,
        r#"{"column":1,"typo":true}"#,
        r#"{"column":1,"header":"A"}"#,
        r#"{"column":18446744073709551616}"#,
    ] {
        assert!(
            AppConfig::from_json(&format!(r#"{{"cage_to_plate":{{"cage":{mapping}}}}}"#)).is_err(),
            "{mapping}"
        );
    }
}

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
fn scoped_saves_preserve_other_saved_settings_not_unsaved_or_stale_values() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new/defaults.json");
    let mut live = AppConfig::default();
    live.cage_to_plate.plate.start_mouse = 12;
    live.plate_to_cage.mouse_label = "UNSAVED {ear_label}".into();
    save_workflow_defaults(&path, WorkflowDefaults::CageToPlate(&live.cage_to_plate)).unwrap();
    let saved = read_config(&path).unwrap();
    assert_eq!(saved.cage_to_plate, live.cage_to_plate);
    assert_eq!(saved.plate_to_cage, PlateToCageConfig::default());

    live.cage_to_plate.plate.start_mouse = 99; // unrelated unsaved edits
    live.plate_to_cage.mouse_label = "{ear_label}/{global_id}".into();
    save_workflow_defaults(&path, WorkflowDefaults::PlateToCage(&live.plate_to_cage)).unwrap();
    let saved = read_config(&path).unwrap();
    assert_eq!(saved.cage_to_plate.plate.start_mouse, 12);
    assert_eq!(saved.plate_to_cage, live.plate_to_cage);

    let mut external = saved.clone();
    external.plate_to_cage.mouse_label = "external {ear_label}".into();
    save_defaults(&path, &external).unwrap();
    save_workflow_defaults(&path, WorkflowDefaults::CageToPlate(&live.cage_to_plate)).unwrap();
    assert_eq!(
        read_config(&path).unwrap().plate_to_cage,
        external.plate_to_cage
    );
    assert_eq!(
        read_config(&path).unwrap().cage_to_plate,
        live.cage_to_plate
    );
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn plate_to_cage_scoped_save_filters_genotype_mappings_and_merges_profiles() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    let mut previous = AppConfig::default();
    previous.plate_to_cage.genotype_display_defaults.insert(
        "untouched".into(),
        GenotypeDisplayConfig {
            name: "Keep me".into(),
            ..Default::default()
        },
    );
    save_defaults(&path, &previous).unwrap();

    let mut live = AppConfig::default();
    live.plate_to_cage.groups = vec![GroupConfig {
        column: "Gene   A".into(),
        name: "Gene A display".into(),
        mode: CategoryMode::All,
        labels: [(cageplate::model::Genotype::Wt, "WT".into())].into(),
        ..Default::default()
    }];
    live.plate_to_cage.output.order =
        vec!["genotype:stale".into(), "cage".into(), "unknown".into()];
    live.plate_to_cage
        .output
        .labels
        .insert("genotype:stale".into(), "Stale genotype".into());
    live.plate_to_cage
        .output
        .labels
        .insert("unknown".into(), "Uncalled".into());

    save_workflow_defaults(&path, WorkflowDefaults::PlateToCage(&live.plate_to_cage)).unwrap();
    let saved = read_config(&path).unwrap();
    assert!(saved.plate_to_cage.groups.is_empty());
    assert_eq!(saved.plate_to_cage.output.order, ["cage", "unknown"]);
    assert!(
        !saved
            .plate_to_cage
            .output
            .labels
            .contains_key("genotype:stale")
    );
    assert_eq!(saved.plate_to_cage.output.labels["unknown"], "Uncalled");
    assert_eq!(
        saved
            .plate_to_cage
            .genotype_display_profile("  GENE  A ")
            .unwrap()
            .name,
        "Gene A display"
    );
    assert_eq!(
        saved
            .plate_to_cage
            .genotype_display_profile("UNTOUCHED")
            .unwrap()
            .name,
        "Keep me"
    );
}

#[test]
fn scoped_save_failures_preserve_defaults_and_do_not_create_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    let settings = CageToPlateConfig::default();
    for contents in [
        "not json",
        r#"{"schema_version":999}"#,
        r#"{"unknown":true}"#,
        r#"{"plate_to_cage":{"tokens":{"hom":"+/+"}}}"#,
    ] {
        fs::write(&path, contents).unwrap();
        assert!(save_workflow_defaults(&path, WorkflowDefaults::CageToPlate(&settings)).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
    }
    save_defaults(&path, &AppConfig::default()).unwrap();
    let original = fs::read(&path).unwrap();
    let mut invalid = settings.clone();
    invalid.plate.rows = 0;
    assert!(save_workflow_defaults(&path, WorkflowDefaults::CageToPlate(&invalid)).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    let missing = temp.path().join("absent/defaults.json");
    assert!(save_workflow_defaults(&missing, WorkflowDefaults::CageToPlate(&invalid)).is_err());
    assert!(!missing.parent().unwrap().exists());
    assert!(save_workflow_defaults(temp.path(), WorkflowDefaults::CageToPlate(&settings)).is_err());
    let blocked = path.join("defaults.json"); // a regular file cannot be a parent
    assert!(save_workflow_defaults(&blocked, WorkflowDefaults::CageToPlate(&settings)).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn scoped_save_permission_failure_keeps_existing_file() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("defaults.json");
    save_defaults(&path, &AppConfig::default()).unwrap();
    let original = fs::read(&path).unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o500)).unwrap();
    // Elevated test runners can bypass directory permissions; only assert denial
    // if the OS actually enforces it for this process.
    let probe = temp.path().join("probe");
    if fs::write(&probe, "probe").is_err() {
        let updated = CageToPlateConfig {
            ears: vec!["custom".into()],
            ..Default::default()
        };
        assert!(save_workflow_defaults(&path, WorkflowDefaults::CageToPlate(&updated)).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
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
    config.plate_to_cage.mouse_label = "{cage_id}/{ear_label}/{global_id}".into();
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
