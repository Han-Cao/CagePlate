use super::{Workflow, output_settings, settings};
use ::cageplate::{
    cage_to_plate::{self, default_ears, read_cages},
    config::{CageLayout, CageToPlateConfig, Traversal, WellStyle},
};
use eframe::egui;

pub(super) fn show(
    ui: &mut egui::Ui,
    state: &mut Workflow,
    config: &mut CageToPlateConfig,
) -> bool {
    state.input_ui(ui, Some(&mut config.has_header));
    let open_config = ui
        .button("Config")
        .on_hover_text("Source mappings, assignment and output settings")
        .clicked();
    state.actions(ui, |table| cage_to_plate::convert(table, config));
    open_config
}

pub(super) fn settings_ui(
    ui: &mut egui::Ui,
    table: Option<&::cageplate::model::InputTable>,
    _output: Option<&::cageplate::model::OutputTable>,
    config: &mut CageToPlateConfig,
) {
    settings::section_title(ui, "Input columns");
    if let Some(table) = table
        && ui
            .add_enabled(
                config.has_header,
                egui::Button::new("Suggest common column mappings"),
            )
            .clicked()
    {
        if let Some(c) = settings::suggest(table, &["Cage #", "Cage", "Cage_id", "Cage ID"]) {
            config.cage = c;
        }
        if let Some(c) = settings::suggest(table, &["QTY", "Quantity", "Count", "Number of mice"]) {
            config.quantity = c;
        }
        if let Some(c) = settings::suggest(table, &["SEX", "Sex"]) {
            config.sex = c;
        }
    }
    settings::column(ui, "Cage ID", &mut config.cage, table, config.has_header);
    settings::column(
        ui,
        "Mouse count",
        &mut config.quantity,
        table,
        config.has_header,
    );
    settings::column(ui, "Sex", &mut config.sex, table, config.has_header);
    settings::section_title(ui, "Output format");
    settings::enum_combo(
        ui,
        "Cage sex layout",
        &mut config.layout,
        &[
            (CageLayout::Newline, "Sex on a new line"),
            (CageLayout::Inline, "Inline sex"),
            (CageLayout::SeparateColumn, "Separate column"),
        ],
    );
    ui.small("New-line/inline layouts show singleton cages inline. Separate column repeats cage and sex on each mouse row.");
    ui.collapsing(settings::section_text("Plate layout"), |ui| {
        egui::Grid::new("plate").num_columns(2).show(ui, |ui| {
            ui.label("Lettered rows (A, B, …)");
            ui.add(egui::DragValue::new(&mut config.plate.rows).range(1..=1024));
            ui.end_row();
            ui.label("Numbered columns");
            ui.add(egui::DragValue::new(&mut config.plate.columns).range(1..=1024));
            ui.end_row();
            let mut row = config.plate.start_row + 1;
            ui.label("Starting row (A=1, B=2, …)");
            ui.add(egui::DragValue::new(&mut row).range(1..=config.plate.rows));
            ui.end_row();
            config.plate.start_row = row.saturating_sub(1);
            let mut col = config.plate.start_column + 1;
            ui.label("Starting column");
            ui.add(egui::DragValue::new(&mut col).range(1..=config.plate.columns));
            ui.end_row();
            config.plate.start_column = col.saturating_sub(1);
            ui.label("First plate number");
            ui.add(egui::DragValue::new(&mut config.plate.start_plate).range(1..=u64::MAX));
            ui.end_row();
            ui.label("First global mouse number");
            ui.add(egui::DragValue::new(&mut config.plate.start_mouse).range(1..=u64::MAX));
            ui.end_row();
        });
        settings::enum_combo(
            ui,
            "Fill direction",
            &mut config.plate.traversal,
            &[
                (Traversal::ColumnWise, "By column: 1A, 1B, …"),
                (Traversal::RowWise, "By row: 1A, 2A, …"),
            ],
        );
        settings::enum_combo(
            ui,
            "Well spelling",
            &mut config.plate.well_style,
            &[
                (WellStyle::NumberLetter, "Number + letter (1A)"),
                (WellStyle::LetterNumber, "Letter + number (A1)"),
            ],
        );
    });
    ui.collapsing(settings::section_text("Ear labels"), |ui| {
        let maximum = table.and_then(|table| read_cages(table, config).ok()).and_then(|cages| cages.iter().map(|c| c.quantity).max()).filter(|n| *n <= ::cageplate::model::MAX_MICE);
        if config.ears.is_empty() {
            ui.label(maximum.map_or("Automatic: map valid input counts to infer the required list.".into(), |n| format!("Automatic: {n} labels needed for the largest cage. R, L, RR, RL, LL, RRR, RRL, RLL, LLL, …")));
            if ui.add_enabled(maximum.is_some(), egui::Button::new("Customize inferred order")).clicked() { config.ears = default_ears(maximum.unwrap_or(0)).unwrap_or_default(); }
        } else {
            ui.label("One label per line, in within-cage order. Reorder lines to change assignment.");
            let mut text = config.ears.join("\n");
            if ui.add(settings::multiline_text_edit(ui, &mut text).id_salt("ears").desired_rows(5)).changed() { config.ears = text.split('\n').map(String::from).collect(); }
            if ui.button("Use automatic labels").clicked() { config.ears.clear(); }
            if ui.add_enabled(maximum.is_some(), egui::Button::new("Regenerate defaults for current input")).clicked() { config.ears = default_ears(maximum.unwrap_or(0)).unwrap_or_default(); }
        }
    });
    let mut columns = vec![
        ("cage", "Cage ID", "Cage"),
        ("sample", "Within-cage ID", "Sample"),
        ("ear", "Ear label", "Ear"),
        ("plate", "Plate ID", "Plate"),
        ("well", "Well ID", "Well"),
        ("mouse_number", "Global ID", "#"),
    ];
    if config.layout == CageLayout::SeparateColumn {
        columns.insert(1, ("sex", "Sex", "SEX"));
    }
    output_settings(ui, &mut config.output, None, &columns);
}
