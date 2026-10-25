use cageplate::{
    cage_to_plate,
    config::*,
    model::*,
    plate_to_cage,
    tsv::{parse_tsv, write_tsv},
};

fn group(column: &str, mode: CategoryMode) -> GroupConfig {
    GroupConfig {
        column: column.into(),
        mode,
        ..Default::default()
    }
}
fn repeated(groups: Vec<GroupConfig>) -> PlateToCageConfig {
    PlateToCageConfig {
        layout: InputLayout::Repeated,
        cage: "Cage".into(),
        sex: Some("Sex".into()),
        within_cage: None,
        plate: None,
        well: None,
        groups,
        ..Default::default()
    }
}

#[test]
fn arbitrary_and_duplicate_source_headers_are_mapped_explicitly() {
    let inventory = parse_tsv("Identity\tN\tGender\nX\t2\tF").unwrap();
    let cfg = CageToPlateConfig {
        cage: "Identity".into(),
        quantity: "N".into(),
        sex: "Gender".into(),
        ..Default::default()
    };
    assert_eq!(
        cage_to_plate::convert(&inventory, &cfg).unwrap().rows.len(),
        2
    );
    let input = parse_tsv("Cage\tSex\tEar\t#\tGene\tGene\nX\tF\tR\t1\t-/-\t+/+").unwrap();
    let mut cfg = repeated(vec![
        group("Gene", CategoryMode::Existing),
        group("Gene", CategoryMode::Existing),
    ]);
    assert!(plate_to_cage::convert(&input, &cfg).is_err());
    cfg.groups[0].column = ColumnRef::from_index(&input, 4).unwrap();
    cfg.groups[1].column = ColumnRef::from_index(&input, 5).unwrap();
    cfg.groups[0].name = "First gene".into();
    cfg.groups[1].name = "Second gene".into();
    let out = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(out.headers[0][3], "First gene, -/-");
    assert_eq!(out.headers[1][3], "Second gene, +/+");
    assert_eq!(out.rows[0][3], "R1");
}

#[test]
fn unknown_in_each_selected_position_routes_only_to_shared_bucket() {
    let input = parse_tsv("Cage\tSex\tEar\t#\tA\tB\tC\nX\tF\tR\t1\t?\t+/+\t+/+\nX\tF\tL\t2\t+/+\t\t+/+\nX\tF\tRR\t3\t+/+\t+/+\t?").unwrap();
    let cfg = repeated(
        ["A", "B", "C"]
            .map(|name| group(name, CategoryMode::Existing))
            .to_vec(),
    );
    let out = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(out.rows[0], ["X", "3", "F", "", "R1, L2, RR3", ""]);
}

#[test]
fn cell_limit_is_checked_before_allocating_summary_buckets() {
    let cfg = repeated(
        (0..7)
            .map(|i| group(&format!("G{i}"), CategoryMode::All))
            .collect(),
    );
    let mice: Vec<_> = (0..1000)
        .map(|i| Mouse {
            cage_id: format!("C{i}"),
            sex: "F".into(),
            ear: "R".into(),
            mouse_number: i + 1,
            within_cage: None,
            plate: None,
            well: None,
            genotypes: vec![Genotype::Hom; 7],
            remark: String::new(),
            source_row: 2,
        })
        .collect();
    assert!(
        plate_to_cage::summarize_cages(&mice, &cfg)
            .unwrap_err()
            .message
            .contains("cell safety limit")
    );
}

#[test]
fn physical_error_lines_account_for_multiline_quoted_records() {
    let input = parse_tsv("Cage\tSex\tEar\t#\tG\tRemarks\r\nX\tF\tR\t1\t-/-\t\"first\r\nsecond\"\r\nX\tF\tL\t2\tTYPO\t\r\n").unwrap();
    assert_eq!(input.source_rows, [2, 4]);
    let mut cfg = repeated(vec![group("G", CategoryMode::Existing)]);
    cfg.remarks = Some("Remarks".into());
    let error = plate_to_cage::convert(&input, &cfg).unwrap_err();
    assert_eq!(error.row, Some(4));
    assert_eq!(error.value.as_deref(), Some("TYPO"));
}

#[test]
fn multi_gene_roundtrip_conserves_every_mouse_and_unknown() {
    let mut inventory = "Cage #\tQTY\tSEX\n".to_string();
    for i in 1..=20 {
        inventory.push_str(&format!(
            "C{i}\t{i}\t{}\n",
            if i % 2 == 0 { "M" } else { "F" }
        ));
    }
    let expanded = cage_to_plate::convert(
        &parse_tsv(&inventory).unwrap(),
        &CageToPlateConfig::default(),
    )
    .unwrap();
    assert_eq!(expanded.rows.len(), 210);
    let mut table = InputTable {
        headers: expanded.headers[0].clone(),
        rows: expanded.rows,
        source_rows: (2..212).collect(),
    };
    table
        .headers
        .extend(["G1".into(), "G2".into(), "G3".into()]);
    for (i, row) in table.rows.iter_mut().enumerate() {
        row.extend([
            ["-/-", "+/-", "+/+", "?"][i % 4].into(),
            ["-/-", "+/-", "+/+"][i % 3].into(),
            "+/+".into(),
        ]);
    }
    for depth in 1..=3 {
        let cfg = PlateToCageConfig {
            groups: (1..=depth)
                .map(|i| {
                    group(
                        &format!("G{i}"),
                        if i == 3 {
                            CategoryMode::Existing
                        } else {
                            CategoryMode::All
                        },
                    )
                })
                .collect(),
            ..Default::default()
        };
        let mice = plate_to_cage::normalize_mouse_rows(&table, &cfg).unwrap();
        let output = plate_to_cage::summarize_cages(&mice, &cfg).unwrap();
        assert_eq!(output.headers.len(), depth);
        assert_eq!(output.rows.len(), 20);
        assert_eq!(
            output
                .rows
                .iter()
                .map(|r| r[1].parse::<usize>().unwrap())
                .sum::<usize>(),
            210
        );
        let mut seen = std::collections::HashSet::new();
        let mut unknown_count = 0;
        for row in &output.rows {
            let mut count = 0;
            for cell in &row[3..row.len() - 1] {
                if !cell.is_empty() {
                    for label in cell.split(", ") {
                        assert!(seen.insert(label));
                        count += 1;
                    }
                }
            }
            assert_eq!(count, row[1].parse::<usize>().unwrap());
            if !row[row.len() - 2].is_empty() {
                unknown_count += row[row.len() - 2].split(", ").count();
            }
        }
        assert_eq!(seen.len(), 210);
        assert_eq!(
            unknown_count,
            mice.iter()
                .filter(|m| m.genotypes.contains(&Genotype::Unknown))
                .count()
        );
    }
}

#[test]
fn existing_sets_are_global_and_display_changes_do_not_change_identity() {
    let input =
        parse_tsv("Cage\tSex\tEar\t#\tA\tB\nX\tF\tR\t1\t-/-\t+/+\nY\tM\tR\t2\t+/+\t-/-").unwrap();
    let mut cfg = repeated(vec![
        group("A", CategoryMode::Existing),
        group("B", CategoryMode::Existing),
    ]);
    let first = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(first.keys.len(), 9); // four Cartesian leaves, not just two observed tuples
    cfg.groups[0].name = "renamed group".into();
    cfg.groups[1].labels.insert(Genotype::Wt, "WT".into());
    let renamed = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(first.keys, renamed.keys);
    assert_eq!(first.rows, renamed.rows);
    cfg.output.order = first.keys.iter().rev().cloned().collect();
    let reordered = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(reordered.keys[0], "remarks");
    assert_eq!(reordered.rows[0].last().unwrap(), "X");
    cfg.output.order.clear();
    cfg.groups.swap(0, 1);
    let swapped = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(
        first.keys.iter().collect::<std::collections::HashSet<_>>(),
        swapped.keys.iter().collect()
    );
}

#[test]
fn configured_tokens_are_exactly_four_and_blanks_still_mean_unknown() {
    let input =
        parse_tsv("Cage\tSex\tEar\t#\tG\nX\tF\tR\t1\tmut\nX\tF\tL\t2\t\nX\tF\tRR\t3\tNA").unwrap();
    let mut cfg = repeated(vec![group("G", CategoryMode::All)]);
    cfg.tokens = GenotypeTokens {
        hom: "mut".into(),
        het: "het".into(),
        wt: "wt".into(),
        unknown: "NA".into(),
    };
    let output = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(output.rows[0][3], "R1");
    assert_eq!(output.rows[0][6], "L2, RR3");
    cfg.tokens.het = "mut".into();
    assert!(cfg.validate().is_err());
}

#[test]
fn remarks_preserve_unicode_tabs_newlines_and_spaces() {
    let input = parse_tsv(
        "Cage\tSex\tEar\t#\tG\tRemarks\nX\tF\tR\t1\t-/-\t\"  α\tβ\nγ  \"\nX\tF\tL\t2\t-/-\tsecond",
    )
    .unwrap();
    let mut cfg = repeated(vec![group("G", CategoryMode::Existing)]);
    cfg.remarks = Some("Remarks".into());
    let output = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(output.rows[0].last().unwrap(), "R:   α\tβ\nγ  ; L: second");
    assert!(
        write_tsv(&output)
            .unwrap()
            .contains("\"R:   α\tβ\nγ  ; L: second\"")
    );
}

#[test]
fn invalid_inventory_and_size_limits_are_rejected() {
    for text in [
        "Cage #\tQTY\tSEX\nA\t0\tF",
        "Cage #\tQTY\tSEX\nA\t-2\tF",
        "Cage #\tQTY\tSEX\nA\t1.5\tF",
        "Cage #\tQTY\tSEX\nA\t1\tF\nA\t1\tF",
        "Cage #\tQTY\tSEX\nA\t100001\tF",
    ] {
        assert!(
            cage_to_plate::convert(&parse_tsv(text).unwrap(), &CageToPlateConfig::default())
                .is_err()
        );
    }
    let mut cfg = CageToPlateConfig::default();
    cfg.plate.start_row = 8;
    assert!(cfg.validate().is_err());
    cfg.plate.start_row = 0;
    cfg.plate.columns = 0;
    assert!(cfg.validate().is_err());
    let cfg = repeated(
        (0..8)
            .map(|i| group(&format!("G{i}"), CategoryMode::All))
            .collect(),
    );
    let mouse = Mouse {
        cage_id: "A".into(),
        sex: "F".into(),
        ear: "R".into(),
        mouse_number: 1,
        within_cage: None,
        plate: None,
        well: None,
        genotypes: vec![Genotype::Hom; 8],
        remark: String::new(),
        source_row: 2,
    };
    assert!(
        plate_to_cage::summarize_cages(&[mouse], &cfg)
            .unwrap_err()
            .message
            .contains("combinations")
    );
}

#[test]
fn duplicate_indices_and_ambiguous_sparse_blocks_are_errors() {
    let mut cfg = repeated(vec![group("G", CategoryMode::Existing)]);
    cfg.within_cage = Some("Sample".into());
    let input =
        parse_tsv("Cage\tSex\tEar\t#\tG\tSample\nA\tF\tR\t1\t-/-\t1\nA\tF\tL\t2\t-/-\t1").unwrap();
    assert!(plate_to_cage::convert(&input, &cfg).is_err());
    cfg.layout = InputLayout::Sparse;
    cfg.sex = None;
    let input =
        parse_tsv("Cage\tEar\t#\tG\tSample\nA\tR\t1\t-/-\t1\nF\tL\t2\t-/-\t2\nM\tRR\t3\t-/-\t3")
            .unwrap();
    assert!(plate_to_cage::convert(&input, &cfg).is_err());
}

#[test]
fn inline_mode_and_configured_optional_label_fields() {
    let mut cfg = repeated(vec![group("G", CategoryMode::Existing)]);
    cfg.layout = InputLayout::Inline;
    cfg.sex = None;
    cfg.plate = Some("Plate".into());
    cfg.well = Some("Well".into());
    cfg.mouse_label = "{plate}/{well}:{ear}".into();
    let input =
        parse_tsv("Cage\tEar\t#\tG\tPlate\tWell\nA (F)\tR\t1\t+/+\t3\t2B\n\tL\t2\t+/+\t3\t2C")
            .unwrap();
    let out = plate_to_cage::convert(&input, &cfg).unwrap();
    assert_eq!(out.rows[0][3], "3/2B:R, 3/2C:L");
    cfg.groups[0].order = vec![Genotype::Wt, Genotype::Hom, Genotype::Het];
    assert!(cfg.validate().is_ok());
    cfg.groups[0].order = vec![Genotype::Unknown];
    assert!(cfg.validate().is_err());
}
