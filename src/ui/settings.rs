//! Shared mapping and compact ordering controls.

use ::cageplate::{
    config::{ColumnRef, GenotypeTokens, GroupConfig},
    model::{Genotype, InputTable},
};
use eframe::egui;

pub(super) fn column(ui: &mut egui::Ui, label: &str, value: &mut ColumnRef, table: &InputTable) {
    let mut selected = Some(value.clone());
    picker(ui, label, &mut selected, table, false);
    if let Some(selected) = selected {
        *value = selected;
    }
}

pub(super) fn optional_column(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ColumnRef>,
    table: &InputTable,
) {
    picker(ui, label, value, table, true);
}

fn picker(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ColumnRef>,
    table: &InputTable,
    optional: bool,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let selected = value.as_ref().and_then(|c| c.resolve(table).ok());
        let text = value
            .as_ref()
            .map_or("Not used".into(), |c| match selected {
                Some(i) => format!("{} [column {}]", table.headers[i], i + 1),
                None => format!("{} (missing / ambiguous)", c.header()),
            });
        egui::ComboBox::from_id_salt(label)
            .selected_text(text)
            .show_ui(ui, |ui| {
                if optional && ui.selectable_label(value.is_none(), "Not used").clicked() {
                    *value = None;
                }
                for (i, header) in table.headers.iter().enumerate() {
                    if ui
                        .selectable_label(
                            selected == Some(i),
                            format!("{header} [column {}]", i + 1),
                        )
                        .clicked()
                    {
                        *value = Some(ColumnRef::from_index(table, i).expect("actual column"));
                    }
                }
            });
    });
}

pub(super) fn suggest(table: &InputTable, aliases: &[&str]) -> Option<ColumnRef> {
    aliases.iter().find_map(|alias| {
        let matches: Vec<_> = table
            .headers
            .iter()
            .enumerate()
            .filter(|(_, h)| h.trim().eq_ignore_ascii_case(alias))
            .map(|(i, _)| i)
            .collect();
        if matches.len() == 1 {
            ColumnRef::from_index(table, matches[0]).ok()
        } else {
            None
        }
    })
}

pub(super) fn enum_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[(T, &str)],
) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(label)
            .selected_text(
                choices
                    .iter()
                    .find(|(v, _)| v == value)
                    .map_or("Select", |(_, s)| *s),
            )
            .show_ui(ui, |ui| {
                for (choice, text) in choices {
                    ui.selectable_value(value, *choice, *text);
                }
            });
    });
}

pub(super) fn genotype_tokens(ui: &mut egui::Ui, tokens: &mut GenotypeTokens) {
    ui.collapsing("Accepted genotype values (exactly four)", |ui| {
        ui.label("Blank always means unknown. Every other value must match one of these tokens; matching is case-sensitive after trimming.");
        egui::Grid::new("tokens").num_columns(2).show(ui, |ui| {
            for (label, value) in [("Hom", &mut tokens.hom), ("Het", &mut tokens.het), ("WT", &mut tokens.wt), ("Unknown", &mut tokens.unknown)] {
                ui.label(label); ui.text_edit_singleline(value); ui.end_row();
            }
        });
    });
}

pub(super) fn group_labels(ui: &mut egui::Ui, group: &mut GroupConfig, tokens: &GenotypeTokens) {
    ui.label("Category order and header names (blank group display name uses source header):");
    let mut movement = None;
    for index in 0..group.order.len() {
        let state = group.order[index];
        ui.push_id(index, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                    movement = Some((index, index - 1));
                }
                if ui
                    .add_enabled(index + 1 < group.order.len(), egui::Button::new("↓"))
                    .clicked()
                {
                    movement = Some((index, index + 1));
                }
                ui.label(match state {
                    Genotype::Hom => "Hom",
                    Genotype::Het => "Het",
                    Genotype::Wt => "WT",
                    Genotype::Unknown => "Unknown",
                });
                let mut label = group.label(state, tokens);
                if ui
                    .add(egui::TextEdit::singleline(&mut label).desired_width(220.0))
                    .changed()
                {
                    group.labels.insert(state, label);
                }
                if ui.small_button("Use default name").clicked() {
                    group.labels.remove(&state);
                }
            });
        });
    }
    if let Some((a, b)) = movement {
        group.order.swap(a, b);
    }
}
