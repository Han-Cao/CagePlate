//! Shared mapping and compact ordering controls.

use super::theme;
use ::cageplate::{
    config::{ColumnRef, GenotypeTokens, GroupConfig},
    model::{Genotype, InputTable},
};
use eframe::egui;

pub(super) fn column(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut ColumnRef,
    table: Option<&InputTable>,
    has_header: bool,
) {
    let mut selected = Some(value.clone());
    source_picker(ui, label, &mut selected, table, has_header, false);
    if let Some(selected) = selected {
        *value = selected;
    }
}

pub(super) fn optional_column(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ColumnRef>,
    table: Option<&InputTable>,
    has_header: bool,
) {
    source_picker(ui, label, value, table, has_header, true);
}

/// Required source selection with an initially empty choice (used by level 1).
pub(super) fn choose_column(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ColumnRef>,
    table: Option<&InputTable>,
) {
    source_picker(ui, label, value, table, true, false);
}

pub(super) fn section_text(title: impl Into<String>) -> egui::RichText {
    egui::RichText::new(title.into())
        .family(egui::FontFamily::Name(theme::SECTION_FONT.into()))
        .strong()
}

pub(super) fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.heading(section_text(title));
}

pub(super) fn text_edit<'a>(_ui: &egui::Ui, value: &'a mut String) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(value)
}

pub(super) fn multiline_text_edit<'a>(_ui: &egui::Ui, value: &'a mut String) -> egui::TextEdit<'a> {
    egui::TextEdit::multiline(value)
}

fn source_picker(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<ColumnRef>,
    table: Option<&InputTable>,
    has_header: bool,
    optional: bool,
) {
    if let Some(table) = table {
        picker(ui, label, value, table, optional);
        if let Some(reference) = value.as_ref()
            && let Err(error) = reference.resolve(table)
        {
            ui.colored_label(ui.visuals().warn_fg_color, error.to_string());
        }
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.label(label);
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt(label)
                    .selected_text(
                        value
                            .as_ref()
                            .map_or("Select column".into(), ColumnRef::display_name),
                    )
                    .show_ui(ui, |_| {});
            });
        });
        ui.small(if has_header {
            "Paste a headered table to select source columns."
        } else {
            "Paste input to select a source column."
        });
    }
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
        let text = value.as_ref().map_or_else(
            || {
                if optional {
                    "Not used".into()
                } else {
                    "Select column".into()
                }
            },
            |c| match selected {
                Some(i) if !table.has_header => format!("Column {}", i + 1),
                Some(i) => format!("{} [column {}]", table.headers[i], i + 1),
                None if c.header() == Some("") => "Select column".into(),
                None => format!("{} (missing / ambiguous)", c.display_name()),
            },
        );
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
                            if table.has_header {
                                format!("{header} [column {}]", i + 1)
                            } else {
                                format!("Column {}", i + 1)
                            },
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
    if !table.has_header {
        return None;
    }
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
    ui.collapsing(section_text("Accepted genotype values"), |ui| {
        ui.label("Blank always means unknown. Every other value must match one of these tokens; matching is case-sensitive after trimming.");
        egui::Grid::new("tokens").num_columns(2).show(ui, |ui| {
            for (label, value) in [("Hom", &mut tokens.hom), ("Het", &mut tokens.het), ("WT", &mut tokens.wt), ("Unknown", &mut tokens.unknown)] {
                ui.label(label);
                ui.add(text_edit(ui, value));
                ui.end_row();
            }
        });
    });
}

/// A normal keyboard-focusable button with a vector arrow, not a font glyph.
pub(super) fn move_button(ui: &mut egui::Ui, up: bool, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let label = if up { "Move up" } else { "Move down" };
        let response = ui
            .add(egui::Button::new("").min_size(egui::vec2(24.0, 24.0)))
            .on_hover_text(label);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
        if ui.is_rect_visible(response.rect) {
            let vector = egui::vec2(0.0, if up { -10.0 } else { 10.0 });
            ui.painter().arrow(
                response.rect.center() - vector * 0.5,
                vector,
                ui.style().interact(&response).fg_stroke,
            );
        }
        response
    })
    .inner
}

pub(super) fn group_labels(ui: &mut egui::Ui, group: &mut GroupConfig, tokens: &GenotypeTokens) {
    ui.label("Category order and header names (blank group display name uses source header):");
    let mut movement = None;
    for index in 0..group.order.len() {
        let state = group.order[index];
        ui.push_id(index, |ui| {
            ui.horizontal(|ui| {
                if move_button(ui, true, index > 0).clicked() {
                    movement = Some((index, index - 1));
                }
                if move_button(ui, false, index + 1 < group.order.len()).clicked() {
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
                    .add(text_edit(ui, &mut label).desired_width(220.0))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ::cageplate::tsv::parse_tsv_with_header;

    fn button_frame(
        ctx: &egui::Context,
        enabled: bool,
        events: Vec<egui::Event>,
    ) -> (egui::Response, Vec<egui::epaint::ClippedShape>) {
        let mut response = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                response = Some(move_button(ui, true, enabled));
            },
        );
        output.textures_delta.clear();
        (response.unwrap(), output.shapes)
    }

    #[test]
    fn vector_buttons_paint_arrows_and_keep_click_keyboard_and_disabled_behavior() {
        let ctx = egui::Context::default();
        super::super::theme::apply(&ctx);
        let (response, shapes) = button_frame(&ctx, true, vec![]);
        assert!(response.enabled());
        let segments: Vec<_> = shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::LineSegment { points, stroke } => Some((points, stroke)),
                _ => None,
            })
            .collect();
        assert_eq!(segments.len(), 3, "arrow shaft and two head segments");
        assert!(segments[0].0[0].y > segments[0].0[1].y, "up arrow");
        assert!(
            segments
                .iter()
                .all(|(_, stroke)| stroke.width > 0.0 && stroke.color.a() > 0)
        );
        let position = response.rect.center();
        let pointer = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        button_frame(
            &ctx,
            true,
            vec![egui::Event::PointerMoved(position), pointer(true)],
        );
        assert!(button_frame(&ctx, true, vec![pointer(false)]).0.clicked());
        ctx.memory_mut(|memory| memory.request_focus(response.id));
        let key = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(button_frame(&ctx, true, vec![key]).0.clicked());
        let (disabled, shapes) = button_frame(&ctx, false, vec![]);
        assert!(!disabled.enabled());
        assert!(
            shapes
                .iter()
                .any(|shape| matches!(shape.shape, egui::Shape::LineSegment { .. }))
        );
        button_frame(&ctx, false, vec![pointer(true)]);
        assert!(!button_frame(&ctx, false, vec![pointer(false)]).0.clicked());
    }

    #[test]
    fn headerless_mapping_is_selected_from_available_columns() {
        use super::super::tests::{frame, pointer, text_point};
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let table = parse_tsv_with_header("A\t2\tF", false).unwrap();
        let mut reference = ColumnRef::from("Old header");
        let output = frame(&ctx, vec![], |ui| {
            column(ui, "Quantity", &mut reference, Some(&table), false)
        });
        let select = text_point(&output, "Old header (missing / ambiguous)");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(select), pointer(select, true)],
            |ui| column(ui, "Quantity", &mut reference, Some(&table), false),
        );
        frame(&ctx, vec![pointer(select, false)], |ui| {
            column(ui, "Quantity", &mut reference, Some(&table), false)
        });
        let output = frame(&ctx, vec![], |ui| {
            column(ui, "Quantity", &mut reference, Some(&table), false)
        });
        let option = text_point(&output, "Column 2");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(option), pointer(option, true)],
            |ui| column(ui, "Quantity", &mut reference, Some(&table), false),
        );
        frame(&ctx, vec![pointer(option, false)], |ui| {
            column(ui, "Quantity", &mut reference, Some(&table), false)
        });
        assert_eq!(reference, ColumnRef::Position { column: 2 });
    }

    #[test]
    fn headerless_controls_work_before_paste_without_guessing_named_mappings() {
        let ctx = egui::Context::default();
        let mut named = ColumnRef::from("Old header");
        let mut position = ColumnRef::Position { column: 7 };
        let mut optional = None;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            column(ui, "Named", &mut named, None, false);
            column(ui, "Position", &mut position, None, false);
            optional_column(ui, "Optional", &mut optional, None, false);
        });
        output.textures_delta.clear();
        assert_eq!(named, ColumnRef::from("Old header"));
        assert_eq!(position, ColumnRef::Position { column: 7 });
        assert_eq!(optional, None);
        let table = parse_tsv_with_header("A\t1", false).unwrap();
        assert!(suggest(&table, &["Column 1"]).is_none());
        let mut output = ctx.run_ui(Default::default(), |ui| {
            column(ui, "Position", &mut position, Some(&table), false);
        });
        output.textures_delta.clear();
        assert_eq!(position, ColumnRef::Position { column: 7 });
        assert!(position.resolve(&table).is_err());
    }
}
