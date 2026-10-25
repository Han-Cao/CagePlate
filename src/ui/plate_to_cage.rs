use super::{Workflow, output_settings, settings};
use ::cageplate::{
    config::{CategoryMode, GroupConfig, InputLayout, PlateToCageConfig},
    plate_to_cage,
};
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, state: &mut Workflow, config: &mut PlateToCageConfig) {
    state.input_ui(ui);
    let before = config.clone();
    ui.heading("2. Map source columns and layout");
    settings::enum_combo(
        ui,
        "Input layout",
        &mut config.layout,
        &[
            (
                InputLayout::Sparse,
                "Sparse cage / second-row sex (also inline singletons)",
            ),
            (InputLayout::Inline, "Inline Cage (Sex), then blanks"),
            (
                InputLayout::Repeated,
                "Repeated cage + separate sex on each row",
            ),
        ],
    );
    if let Some(table) = &state.parsed {
        if ui.button("Suggest common column mappings").clicked() {
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
            config.within_cage =
                settings::suggest(table, &["Sample", "Within cage", "Within_cage"]);
            config.plate = settings::suggest(table, &["Plate", "Plate number"]);
            config.well = settings::suggest(table, &["Well"]);
            config.remarks = settings::suggest(table, &["Remarks", "Remark", "Notes"]);
        }
        settings::column(ui, "Cage / cage display", &mut config.cage, table);
        settings::column(ui, "Ear", &mut config.ear, table);
        settings::column(ui, "Global mouse number", &mut config.mouse_number, table);
        settings::optional_column(ui, "Separate sex", &mut config.sex, table);
        settings::optional_column(ui, "Within-cage index", &mut config.within_cage, table);
        settings::optional_column(ui, "Plate", &mut config.plate, table);
        settings::optional_column(ui, "Well", &mut config.well, table);
        settings::optional_column(ui, "Remarks", &mut config.remarks, table);
    } else {
        ui.label("Paste a valid table to select its source columns.");
    }
    ui.heading("3. Configure genotype hierarchy");
    ui.label("Order genes from outermost to innermost. Existing uses observed known calls; All also includes absent known categories.");
    let mut movement = None;
    let mut remove = None;
    let count = config.groups.len();
    for (index, group) in config.groups.iter_mut().enumerate() {
        ui.push_id(("group", index), |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Level {}: {}", index + 1, group.column.header()));
                    if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                        movement = Some((index, index - 1));
                    }
                    if ui
                        .add_enabled(index + 1 < count, egui::Button::new("↓"))
                        .clicked()
                    {
                        movement = Some((index, index + 1));
                    }
                    if ui.button("Remove gene").clicked() {
                        remove = Some(index);
                    }
                });
                if let Some(table) = &state.parsed {
                    settings::column(ui, "Genotype source", &mut group.column, table);
                }
                settings::enum_combo(
                    ui,
                    "Include genotypes",
                    &mut group.mode,
                    &[
                        (CategoryMode::Existing, "Existing"),
                        (CategoryMode::All, "All (Hom, Het, WT)"),
                    ],
                );
                ui.collapsing("Names and category order", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Group display name");
                        ui.text_edit_singleline(&mut group.name);
                    });
                    settings::group_labels(ui, group, &config.tokens);
                });
            });
        });
    }
    if let Some(index) = remove {
        config.groups.remove(index);
    } else if let Some((a, b)) = movement {
        config.groups.swap(a, b);
    }
    if let Some(table) = &state.parsed {
        let mut add = None;
        ui.add_enabled_ui(config.groups.len() < ::cageplate::model::MAX_GROUPS, |ui| {
            egui::ComboBox::from_id_salt("add_gene")
                .selected_text("Add genotype column…")
                .show_ui(ui, |ui| {
                    for (i, header) in table.headers.iter().enumerate() {
                        if !config
                            .groups
                            .iter()
                            .any(|g| g.column.resolve(table).ok() == Some(i))
                            && ui
                                .selectable_label(false, format!("{header} [column {}]", i + 1))
                                .clicked()
                        {
                            add = Some(i);
                        }
                    }
                });
        });
        if let Some(i) = add {
            config.groups.push(GroupConfig {
                column: ::cageplate::config::ColumnRef::from_index(table, i)
                    .expect("actual column"),
                ..Default::default()
            });
        }
    }
    settings::genotype_tokens(ui, &mut config.tokens);
    ui.collapsing("Mouse labels and sparse sex markers", |ui| {
        ui.label("Mouse label template (default {ear}{mouse_number}):"); ui.text_edit_singleline(&mut config.mouse_label);
        ui.small("Fields: {ear}, {mouse_number}, {within_cage}, {plate}, {well}, {cage}, {sex}. Optional fields must be mapped and present.");
        ui.label("Sparse-layout sex markers, one per line:");
        let mut text = config.sex_tokens.join("\n");
        if ui.add(egui::TextEdit::multiline(&mut text).id_salt("sex_tokens").desired_rows(2)).changed() { config.sex_tokens = text.split('\n').map(String::from).collect(); }
    });
    output_settings(
        ui,
        &mut config.output,
        state.output.as_ref(),
        &[
            ("cage", "Cage #"),
            ("quantity", "QTY"),
            ("sex", "SEX"),
            ("unknown", "?"),
            ("remarks", "Remarks"),
        ],
    );
    if *config != before {
        state.invalidate();
    }
    state.actions(ui, |table| plate_to_cage::convert(table, config));
}
