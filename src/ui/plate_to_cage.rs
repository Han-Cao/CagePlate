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
fn genotype_sources(ui: &mut egui::Ui, groups: &mut Vec<GroupConfig>, table: Option<&InputTable>) {
    if groups.is_empty() {
        let mut first = None;
        ui.horizontal(|ui| {
            settings::choose_column(ui, "Genotype (level 1)", &mut first, table);
            ui.add_enabled(false, egui::Button::new("Add genotype"));
        });
        if let Some(column) = first {
            groups.push(GroupConfig {
                column,
                ..Default::default()
            });
        }
        return;
    }
    let can_add = groups.len() < MAX_GROUPS
        && table.is_some_and(|table| {
            groups
                .iter()
                .all(|group| group.column.resolve(table).is_ok())
        });
    let count = groups.len();
    let mut add = false;
    let mut remove = None;
    let mut movement = None;
    for (index, group) in groups.iter_mut().enumerate() {
        ui.push_id(("genotype_source", index), |ui| {
            ui.horizontal(|ui| {
                settings::column(
                    ui,
                    &format!("Genotype (level {})", index + 1),
                    &mut group.column,
                    table,
                    true,
                );
                if index == 0
                    && ui
                        .add_enabled(can_add, egui::Button::new("Add genotype"))
                        .clicked()
                {
                    add = true;
                }
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
    }
    if let Some(index) = remove {
        groups.remove(index);
    } else if let Some((a, b)) = movement {
        groups.swap(a, b);
    } else if add {
        // An empty source requires an explicit selection before Apply/save.
        groups.push(GroupConfig {
            column: ColumnRef::Name(String::new()),
            ..Default::default()
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
    genotype_sources(ui, &mut config.groups, table);

    settings::section_title(ui, "Genotypes");
    settings::genotype_tokens(ui, &mut config.tokens);
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
                        ui.text_edit_singleline(&mut group.name);
                    });
                    settings::group_labels(ui, group, &config.tokens);
                });
            });
        });
    }
    ui.collapsing(settings::section_text("Mouse labels and sparse sex markers"), |ui| {
        ui.label("Mouse label template (default {ear}{mouse_number}):");
        ui.text_edit_singleline(&mut config.mouse_label);
        ui.small("Fields: {ear}, {mouse_number}, {within_cage}, {plate}, {well}, {cage}, {sex}. Optional fields must be mapped and present.");
        ui.label("Sparse-layout sex markers, one per line:");
        let mut text = config.sex_tokens.join("\n");
        if ui.add(egui::TextEdit::multiline(&mut text).id_salt("sex_tokens").desired_rows(2)).changed() {
            config.sex_tokens = text.split('\n').map(String::from).collect();
        }
    });
    output_settings(
        ui,
        &mut config.output,
        output,
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
    use ::cageplate::tsv::parse_tsv;

    fn click_source_control(
        ctx: &egui::Context,
        groups: &mut Vec<GroupConfig>,
        table: &InputTable,
        label: &str,
    ) {
        let output = frame(ctx, vec![], |ui| genotype_sources(ui, groups, Some(table)));
        let pos = text_point(&output, label);
        frame(
            ctx,
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            |ui| genotype_sources(ui, groups, Some(table)),
        );
        frame(ctx, vec![pointer(pos, false)], |ui| {
            genotype_sources(ui, groups, Some(table))
        });
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
            genotype_sources(ui, &mut groups, Some(&table))
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
            |ui| genotype_sources(ui, &mut groups, Some(&table)),
        );
        frame(&ctx, vec![pointer(remove, false)], |ui| {
            genotype_sources(ui, &mut groups, Some(&table))
        });
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Custom A");
    }
}
