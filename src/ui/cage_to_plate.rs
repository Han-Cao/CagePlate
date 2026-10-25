use super::{Workflow, output_settings, settings};
use ::cageplate::{
    cage_to_plate::{self, default_ears, read_cages},
    config::{CageLayout, CageToPlateConfig, Traversal, WellStyle},
};
use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, state: &mut Workflow, config: &mut CageToPlateConfig) {
    state.input_ui(ui);
    let before = config.clone();
    ui.heading("2. Map source columns");
    if let Some(table) = &state.parsed {
        if ui.button("Suggest common column mappings").clicked() {
            if let Some(c) = settings::suggest(table, &["Cage #", "Cage", "Cage_id", "Cage ID"]) {
                config.cage = c;
            }
            if let Some(c) =
                settings::suggest(table, &["QTY", "Quantity", "Count", "Number of mice"])
            {
                config.quantity = c;
            }
            if let Some(c) = settings::suggest(table, &["SEX", "Sex"]) {
                config.sex = c;
            }
        }
        settings::column(ui, "Cage ID", &mut config.cage, table);
        settings::column(ui, "Mouse count", &mut config.quantity, table);
        settings::column(ui, "Sex", &mut config.sex, table);
    } else {
        ui.label("Paste a valid table to select its source columns.");
    }
    ui.heading("3. Configure assignment");
    settings::enum_combo(
        ui,
        "Cage display",
        &mut config.layout,
        &[
            (CageLayout::Newline, "Sex on second mouse row"),
            (CageLayout::Inline, "Cage (Sex)"),
        ],
    );
    ui.small("Singleton cages always use Cage (Sex). Cages may span plates.");
    ui.collapsing("Plate dimensions, order, and starting position", |ui| {
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
    ui.collapsing("Ear labels and their order", |ui| {
        let maximum = state.parsed.as_ref().and_then(|table| read_cages(table, config).ok()).and_then(|cages| cages.iter().map(|c| c.quantity).max()).filter(|n| *n <= ::cageplate::model::MAX_MICE);
        if config.ears.is_empty() {
            ui.label(maximum.map_or("Automatic: map valid input counts to infer the required list.".into(), |n| format!("Automatic: {n} labels needed for the largest cage. R, L, RR, RL, LL, RRR, RRL, RLL, LLL, …")));
            if ui.add_enabled(maximum.is_some(), egui::Button::new("Customize inferred order")).clicked() { config.ears = default_ears(maximum.unwrap_or(0)).unwrap_or_default(); }
        } else {
            ui.label("One label per line, in within-cage order. Reorder lines to change assignment.");
            let mut text = config.ears.join("\n");
            if ui.add(egui::TextEdit::multiline(&mut text).id_salt("ears").desired_rows(5)).changed() { config.ears = text.split('\n').map(String::from).collect(); }
            if ui.button("Use automatic labels").clicked() { config.ears.clear(); }
            if ui.add_enabled(maximum.is_some(), egui::Button::new("Regenerate defaults for current input")).clicked() { config.ears = default_ears(maximum.unwrap_or(0)).unwrap_or_default(); }
        }
    });
    output_settings(
        ui,
        &mut config.output,
        state.output.as_ref(),
        &[
            ("cage", "Cage"),
            ("sample", "Sample"),
            ("ear", "Ear"),
            ("plate", "Plate"),
            ("well", "Well"),
            ("mouse_number", "#"),
        ],
    );
    if *config != before {
        state.invalidate();
    }
    state.actions(ui, |table| cage_to_plate::convert(table, config));
}
