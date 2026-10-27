use super::{Workflow, output_settings, settings};
use ::cageplate::{
    config::{CategoryMode, ColumnRef, GroupConfig, InputLayout, PlateToCageConfig},
    model::{InputTable, MAX_GROUPS, OutputTable},
    plate_to_cage,
};
use eframe::egui;

pub(super) fn show(
    ui: &mut egui::Ui,
    state: &mut Workflow,
    config: &mut PlateToCageConfig,
) -> bool {
    state.input_ui(ui, None);
    let open_config = ui
        .button("Config")
        .on_hover_text("Source mappings, genotypes and output settings")
        .clicked();
    state.actions(ui, |table| plate_to_cage::convert(table, config));
    open_config
}

// Source selection and hierarchy operations live together. The cards below
// follow this same vector, so names/category settings travel with each gene.
fn genotype_sources(ui: &mut egui::Ui, config: &mut PlateToCageConfig, table: Option<&InputTable>) {
    if config.groups.is_empty() {
        let mut first = None;
        ui.horizontal(|ui| {
            settings::choose_column(ui, "Genotype (level 1)", &mut first, table);
        });
        if let Some(column) = first {
            let mut group = GroupConfig {
                column,
                ..Default::default()
            };
            let profile = group
                .column
                .header()
                .and_then(|name| config.genotype_display_profile(name).cloned());
            if let Some(profile) = profile {
                profile.apply_to(&mut group);
            }
            config.groups.push(group);
        }
        return;
    }
    let can_add = config.groups.len() < MAX_GROUPS
        && table.is_some_and(|table| {
            config
                .groups
                .iter()
                .all(|group| group.column.resolve(table).is_ok())
        });
    let count = config.groups.len();
    let mut changed_sources = vec![];
    let mut remove = None;
    let mut movement = None;
    for index in 0..count {
        let before = config.groups[index].column.clone();
        ui.push_id(("genotype_source", index), |ui| {
            ui.horizontal(|ui| {
                settings::column(
                    ui,
                    &format!("Genotype (level {})", index + 1),
                    &mut config.groups[index].column,
                    table,
                    true,
                );
                if settings::move_button(ui, true, index > 0).clicked() {
                    movement = Some((index, index - 1));
                }
                if settings::move_button(ui, false, index + 1 < count).clicked() {
                    movement = Some((index, index + 1));
                }
                if ui.button("Remove").clicked() {
                    remove = Some(index);
                }
            });
        });
        if config.groups[index].column != before {
            changed_sources.push(index);
        }
    }
    if let Some(index) = remove {
        config.groups.remove(index);
    } else if let Some((a, b)) = movement {
        config.groups.swap(a, b);
    } else {
        for index in changed_sources {
            let profile = config.groups[index]
                .column
                .header()
                .and_then(|name| config.genotype_display_profile(name).cloned());
            if let Some(profile) = profile {
                profile.apply_to(&mut config.groups[index]);
            }
        }
        ui.add_enabled_ui(can_add, |ui| {
            if ui.button("Add genotype").clicked() {
                config.groups.push(GroupConfig {
                    column: ColumnRef::Name(String::new()),
                    ..Default::default()
                });
            }
        });
    }
}

pub(super) fn settings_ui(
    ui: &mut egui::Ui,
    table: Option<&InputTable>,
    output: Option<&OutputTable>,
    config: &mut PlateToCageConfig,
) {
    let table = table.filter(|table| table.has_header);
    settings::section_title(ui, "Input columns");
    settings::enum_combo(
        ui,
        "Input cage sex layout",
        &mut config.layout,
        &[
            (InputLayout::Sparse, "Sex on a new line"),
            (InputLayout::Inline, "Inline sex"),
            (InputLayout::Repeated, "Separate column"),
        ],
    );
    if let Some(table) = table
        && ui.button("Suggest common column mappings").clicked()
    {
        if let Some(c) = settings::suggest(table, &["Cage", "Cage #", "Cage_id", "Cage ID"]) {
            config.cage = c;
        }
        if let Some(c) = settings::suggest(table, &["Ear", "Ear label"]) {
            config.ear = c;
        }
        if let Some(c) = settings::suggest(table, &["#", "Mouse number", "Mouse_number"]) {
            config.mouse_number = c;
        }
        config.sex = settings::suggest(table, &["SEX", "Sex"]);
        config.within_cage = settings::suggest(table, &["Sample", "Within cage", "Within_cage"]);
        config.plate = settings::suggest(table, &["Plate", "Plate number"]);
        config.well = settings::suggest(table, &["Well"]);
        config.remarks = settings::suggest(table, &["Remarks", "Remark", "Notes"]);
    }
    settings::column(ui, "Cage ID", &mut config.cage, table, true);
    settings::optional_column(ui, "Within-cage ID", &mut config.within_cage, table, true);
    settings::column(ui, "Ear label", &mut config.ear, table, true);
    settings::optional_column(ui, "Plate ID", &mut config.plate, table, true);
    settings::optional_column(ui, "Well ID", &mut config.well, table, true);
    settings::column(ui, "Global ID", &mut config.mouse_number, table, true);
    settings::optional_column(ui, "Remarks", &mut config.remarks, table, true);
    settings::optional_column(ui, "Sex", &mut config.sex, table, true);
    genotype_sources(ui, config, table);

    settings::section_title(ui, "Genotypes");
    if config.groups.is_empty() {
        ui.label("Select a genotype in Input columns to configure it.");
    }
    for (index, group) in config.groups.iter_mut().enumerate() {
        if group.column.header() == Some("") {
            continue;
        }
        ui.push_id(("genotype_card", index), |ui| {
            ui.group(|ui| {
                ui.label(settings::section_text(format!(
                    "Genotype (level {}): {}",
                    index + 1,
                    group.column.display_name()
                )));
                settings::enum_combo(
                    ui,
                    "Include genotypes",
                    &mut group.mode,
                    &[
                        (CategoryMode::Existing, "Existing"),
                        (CategoryMode::All, "All (Hom, Het, WT)"),
                    ],
                );
                ui.collapsing(settings::section_text("Display names"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Group display name");
                        ui.add(settings::text_edit(ui, &mut group.name));
                    });
                    settings::group_labels(ui, group, &config.tokens);
                });
            });
        });
    }
    settings::genotype_tokens(ui, &mut config.tokens);
    ui.collapsing(settings::section_text("Mouse labeling"), |ui| {
        ui.label("Mouse ID");
        ui.add(settings::text_edit(ui, &mut config.mouse_label));
        ui.small("Fields: {cage_id}, {within_cage_id}, {ear_label}, {global_id}, {sex}.");
        ui.label("Allowed sex");
        let mut text = config.sex_tokens.join("\n");
        if ui
            .add(
                settings::multiline_text_edit(ui, &mut text)
                    .id_salt("sex_tokens")
                    .desired_rows(2),
            )
            .changed()
        {
            config.sex_tokens = text.split('\n').map(String::from).collect();
        }
    });
    let preview_output = table.and_then(|table| plate_to_cage::convert(table, config).ok());
    let current_output = if table.is_some() {
        preview_output.as_ref()
    } else {
        output
    };
    output_settings(
        ui,
        &mut config.output,
        current_output,
        &[
            ("cage", "Cage ID", "Cage #"),
            ("quantity", "Quantity", "QTY"),
            ("sex", "Sex", "SEX"),
            ("unknown", "Unknown genotype", "?"),
            ("remarks", "Remarks", "Remarks"),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::super::{
        tests::{frame, pointer, text_point},
        theme,
    };
    use super::*;
    use ::cageplate::{config::GenotypeDisplayConfig, tsv::parse_tsv};

    fn genotype_sources_for_groups(
        ui: &mut egui::Ui,
        groups: &mut Vec<GroupConfig>,
        table: Option<&InputTable>,
    ) {
        let mut config = PlateToCageConfig {
            groups: std::mem::take(groups),
            ..Default::default()
        };
        genotype_sources(ui, &mut config, table);
        *groups = config.groups;
    }

    fn click_config_source_control(
        ctx: &egui::Context,
        config: &mut PlateToCageConfig,
        table: &InputTable,
        label: &str,
    ) {
        let output = frame(ctx, vec![], |ui| genotype_sources(ui, config, Some(table)));
        let pos = text_point(&output, label);
        frame(
            ctx,
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            |ui| genotype_sources(ui, config, Some(table)),
        );
        frame(ctx, vec![pointer(pos, false)], |ui| {
            genotype_sources(ui, config, Some(table))
        });
    }

    fn click_source_control(
        ctx: &egui::Context,
        groups: &mut Vec<GroupConfig>,
        table: &InputTable,
        label: &str,
    ) {
        let output = frame(ctx, vec![], |ui| {
            genotype_sources_for_groups(ui, groups, Some(table))
        });
        let pos = text_point(&output, label);
        frame(
            ctx,
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            |ui| genotype_sources_for_groups(ui, groups, Some(table)),
        );
        frame(ctx, vec![pointer(pos, false)], |ui| {
            genotype_sources_for_groups(ui, groups, Some(table))
        });
    }

    #[test]
    fn genotype_selection_reuses_normalized_display_defaults() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let table = parse_tsv("Cage\tGene A\nA (F)\t+/+\n").unwrap();
        let mut config = PlateToCageConfig::default();
        config.genotype_display_defaults.insert(
            "gene a".into(),
            GenotypeDisplayConfig {
                name: "Custom A".into(),
                mode: CategoryMode::All,
                ..Default::default()
            },
        );
        click_config_source_control(&ctx, &mut config, &table, "Select column");
        click_config_source_control(&ctx, &mut config, &table, "Gene A [column 2]");
        assert_eq!(config.groups.len(), 1);
        assert_eq!(config.groups[0].name, "Custom A");
        assert_eq!(config.groups[0].mode, CategoryMode::All);
    }

    #[test]
    fn output_preview_follows_live_genotype_display_settings() {
        let table = parse_tsv("Cage\tEar\t#\tGene A\nA (F)\tR\t1\t-/-\n").unwrap();
        let mut config = PlateToCageConfig {
            within_cage: None,
            plate: None,
            well: None,
            layout: InputLayout::Inline,
            groups: vec![GroupConfig {
                column: "Gene A".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let preview = plate_to_cage::convert(&table, &config).unwrap();
        assert!(
            preview
                .headers
                .last()
                .is_some_and(|headers| headers.contains(&"Gene A, -/-".into()))
        );
        config.groups[0].name = "Renamed".into();
        let preview = plate_to_cage::convert(&table, &config).unwrap();
        assert!(
            preview
                .headers
                .last()
                .is_some_and(|headers| headers.contains(&"Renamed, -/-".into()))
        );
    }

    #[test]
    fn genotype_selection_adds_levels_and_matching_cards_in_input_columns() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let table = parse_tsv("Cage\tGene A\tGene B\nA (F)\t+/+\t-/-").unwrap();
        let mut groups = vec![];
        click_source_control(&ctx, &mut groups, &table, "Select column");
        click_source_control(&ctx, &mut groups, &table, "Gene A [column 2]");
        assert_eq!(groups.len(), 1);
        groups[0].name = "Custom A".into();
        click_source_control(&ctx, &mut groups, &table, "Add genotype");
        assert_eq!(groups.len(), 2);
        assert!(
            groups[1].validate().is_err(),
            "new level requires explicit source selection"
        );
        click_source_control(&ctx, &mut groups, &table, "Select column");
        click_source_control(&ctx, &mut groups, &table, "Gene B [column 3]");
        assert_eq!(groups[1].column, ColumnRef::from("Gene B"));
        let mut config = PlateToCageConfig {
            groups: groups.clone(),
            ear: "Gene A".into(),
            mouse_number: "Gene B".into(),
            within_cage: None,
            plate: None,
            well: None,
            ..Default::default()
        };
        let output = frame(&ctx, vec![], |ui| {
            settings_ui(ui, Some(&table), None, &mut config)
        });
        text_point(&output, "Genotype (level 1): Gene A");
        text_point(&output, "Genotype (level 2): Gene B");
        for old in [
            "Genotype source",
            "Add genotype column",
            "Names and category order",
            "Configure genotype hierarchy",
        ] {
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == old)));
        }
        // Removal is in the input mapping rows, not in the display cards, and
        // preserves the surviving gene's settings while levels are renumbered.
        let output = frame(&ctx, vec![], |ui| {
            genotype_sources_for_groups(ui, &mut groups, Some(&table))
        });
        let remove = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Remove" => {
                    Some(text.pos + egui::vec2(3.0, 3.0))
                }
                _ => None,
            })
            .nth(1)
            .expect("second genotype removal");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(remove), pointer(remove, true)],
            |ui| genotype_sources_for_groups(ui, &mut groups, Some(&table)),
        );
        frame(&ctx, vec![pointer(remove, false)], |ui| {
            genotype_sources_for_groups(ui, &mut groups, Some(&table))
        });
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Custom A");
    }
}
