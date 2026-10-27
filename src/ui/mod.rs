//! Native clipboard UI. Conversion logic remains in the shared library.

mod cage_to_plate;
mod config_dialog;
mod plate_to_cage;
mod settings;
mod theme;

use ::cageplate::{
    Result,
    config::{self, AppConfig, OutputConfig},
    model::{InputTable, OutputTable},
    tsv::{infer_has_header, parse_tsv_with_header, write_tsv, write_tsv_without_headers},
};
use eframe::egui;
use std::path::PathBuf;

#[derive(Default)]
struct Workflow {
    input: String,
    parsed: Option<InputTable>,
    input_error: Option<String>,
    output: Option<OutputTable>,
    output_tsv: String,
    output_tsv_without_headers: String,
    conversion_error: Option<String>,
    stale: bool,
}

impl Workflow {
    fn invalidate(&mut self) {
        self.stale = true;
        self.conversion_error = None;
    }

    fn reparse(&mut self, has_header: bool) {
        self.invalidate();
        match parse_tsv_with_header(&self.input, has_header) {
            Ok(table) => {
                self.parsed = Some(table);
                self.input_error = None;
            }
            Err(error) => {
                self.parsed = None;
                self.input_error = if self.input.trim().is_empty() {
                    None
                } else {
                    Some(error.to_string())
                };
            }
        }
    }

    // Only Cage to Plate exposes/infer its header option. Plate to Cage passes
    // None, always parses headers, and has no headerless UI path.
    fn input_ui(&mut self, ui: &mut egui::Ui, header_option: Option<&mut bool>) {
        settings::section_title(ui, "Input");
        let infer_on_paste = header_option.is_some();
        let mut has_header = header_option.as_deref().copied().unwrap_or(true);
        let mut replace = false;
        let mut clear = false;
        let header_changed = infer_on_paste
            && ui
                .checkbox(&mut has_header, "Input includes header row")
                .changed();
        ui.horizontal_wrapped(|ui| {
            replace = ui.button("Paste / replace").on_hover_text("Replace input with clipboard text. You can also focus the box and press Ctrl+V.").clicked();
            clear = ui.button("Clear").clicked();
        });
        if replace || clear {
            self.input.clear();
        }
        let response = ui.add(
            settings::multiline_text_edit(ui, &mut self.input)
                .id_salt("input")
                .code_editor()
                .desired_rows(6)
                .desired_width(f32::INFINITY),
        );
        if replace {
            response.request_focus();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        if response.changed()
            && infer_on_paste
            && ui.ctx().input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Paste(_)))
            })
            && let Ok(Some(inferred)) = infer_has_header(&self.input)
        {
            has_header = inferred;
        }
        if let Some(option) = header_option {
            *option = has_header;
        }
        if response.changed() || replace || clear || header_changed {
            self.reparse(has_header);
        }
        if let Some(error) = &self.input_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        if let Some(table) = &self.parsed {
            ui.label(format!(
                "{} records · {} columns",
                table.rows.len(),
                table.headers.len()
            ));
        }
    }

    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        convert: impl FnOnce(&InputTable) -> Result<OutputTable>,
    ) {
        ui.separator();
        let mut run = false;
        ui.horizontal(|ui| {
            run = ui
                .add_enabled(self.parsed.is_some(), egui::Button::new("Convert"))
                .clicked();
        });
        if run {
            match self
                .parsed
                .as_ref()
                .map(convert)
                .expect("enabled with input")
                .and_then(|output| {
                    Ok((
                        write_tsv(&output)?,
                        write_tsv_without_headers(&output)?,
                        output,
                    ))
                }) {
                Ok((text, body, output)) => {
                    self.output_tsv = text;
                    self.output_tsv_without_headers = body;
                    self.output = Some(output);
                    self.stale = false;
                    self.conversion_error = None;
                }
                Err(error) => {
                    self.invalidate();
                    self.conversion_error = Some(error.to_string());
                }
            }
        }
        if let Some(error) = &self.conversion_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        settings::section_title(ui, "Output");
        ui.horizontal_wrapped(|ui| {
            let ready = self.output.is_some() && !self.stale;
            if ui
                .add_enabled(ready, egui::Button::new("Copy output (with headers)"))
                .clicked()
            {
                ui.ctx().copy_text(self.output_tsv.clone());
            }
            if ui
                .add_enabled(ready, egui::Button::new("Copy output (without headers)"))
                .clicked()
            {
                ui.ctx().copy_text(self.output_tsv_without_headers.clone());
            }
        });
        if self.stale && self.output.is_some() {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Preview is stale. Convert again before copying.",
            );
        }
        if let Some(table) = &self.output {
            preview(ui, table);
        }
    }
}

fn preview(ui: &mut egui::Ui, table: &OutputTable) {
    ui.label(format!(
        "{} rows · {} columns · hover a cell to read its full contents",
        table.rows.len(),
        table.keys.len()
    ));
    let width = 140.0;
    let height = 24.0;
    egui::ScrollArea::horizontal()
        .id_salt("preview_horizontal")
        .show(ui, |ui| {
            ui.set_min_width(table.keys.len() as f32 * (width + ui.spacing().item_spacing.x));
            for header in &table.headers {
                ui.horizontal(|ui| {
                    for cell in header {
                        ui.add_sized(
                            [width, height],
                            egui::Label::new(egui::RichText::new(cell).strong()).truncate(),
                        )
                        .on_hover_text(cell);
                    }
                });
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("preview_rows")
                .max_height(320.0)
                .show_rows(ui, height, table.rows.len(), |ui, range| {
                    for index in range {
                        ui.push_id(index, |ui| {
                            ui.horizontal(|ui| {
                                for cell in &table.rows[index] {
                                    ui.add_sized(
                                        [width, height],
                                        egui::Label::new(cell).truncate(),
                                    )
                                    .on_hover_text(cell);
                                }
                            });
                        });
                    }
                });
        });
}

pub(super) struct CagePlateApp {
    config: AppConfig,
    config_path: Option<PathBuf>,
    warning: Option<String>,
    dialog: Option<config_dialog::ConfigDialog>,
    tab: usize,
    cage: Workflow,
    plate: Workflow,
}

impl CagePlateApp {
    pub(super) fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        let (config, path, warning) = match config::default_path() {
            Ok(path) => {
                let loaded = config::load_defaults(&path);
                (loaded.config, Some(path), loaded.warning)
            }
            Err(error) => (AppConfig::default(), None, Some(error.to_string())),
        };
        Self {
            config,
            config_path: path,
            warning,
            dialog: None,
            tab: 0,
            cage: Workflow::default(),
            plate: Workflow::default(),
        }
    }
}

impl eframe::App for CagePlateApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut open_config = false;
        theme::root_frame(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.tab, 0, "Cage to Plate");
                ui.selectable_value(&mut self.tab, 1, "Plate to Cage");
            });
            if let Some(warning) = &self.warning {
                let mut dismiss = false;
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(ui.visuals().warn_fg_color, warning);
                    dismiss = ui.small_button("Dismiss").clicked();
                });
                if dismiss {
                    self.warning = None;
                }
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt(("workflow", self.tab))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.push_id(self.tab, |ui| {
                        if self.tab == 0 {
                            open_config = cage_to_plate::show(
                                ui,
                                &mut self.cage,
                                &mut self.config.cage_to_plate,
                            );
                        } else {
                            open_config = plate_to_cage::show(
                                ui,
                                &mut self.plate,
                                &mut self.config.plate_to_cage,
                            );
                        }
                    });
                });
        });
        if open_config && self.dialog.is_none() {
            self.dialog = Some(config_dialog::ConfigDialog::new(&self.config, self.tab));
        }
        if let Some(mut dialog) = self.dialog.take() {
            let (state, has_header) = if dialog.is_cage() {
                (&self.cage, self.config.cage_to_plate.has_header)
            } else {
                (&self.plate, self.config.plate_to_cage.has_header)
            };
            let action = dialog.show(ui.ctx(), state, has_header, self.config_path.as_deref());
            let close = action.is_some_and(|action| {
                dialog.handle_action(
                    action,
                    &mut self.config,
                    &mut self.cage,
                    &mut self.plate,
                    self.config_path.as_deref(),
                )
            });
            if !close {
                self.dialog = Some(dialog);
            }
        }
    }
}

fn output_column_default_label(table: &OutputTable, index: usize) -> String {
    table
        .headers
        .iter()
        .filter_map(|row| row.get(index))
        .filter(|cell| !cell.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ")
}

// Output controls work even before a conversion for the fixed metadata columns.
fn output_settings(
    ui: &mut egui::Ui,
    config: &mut OutputConfig,
    output: Option<&OutputTable>,
    base: &[(&str, &str, &str)],
) {
    ui.collapsing(settings::section_text("Output columns"), |ui| {
        let mut keys: Vec<String> = output.map_or_else(
            || base.iter().map(|(key, _, _)| key.to_string()).collect(),
            |table| table.keys.clone(),
        );
        for (key, _, _) in base {
            if !keys.iter().any(|k| k == key) {
                keys.push((*key).into());
            }
        }
        let natural = keys.clone();
        keys.sort_by_key(|key| {
            config
                .order
                .iter()
                .position(|k| k == key)
                .unwrap_or(config.order.len() + natural.iter().position(|k| k == key).unwrap_or(0))
        });
        let mut movement = None;
        for (index, key) in keys.iter().enumerate() {
            let metadata = base.iter().find(|(k, _, _)| *k == key);
            let meaning = metadata.map_or("Genotype", |(_, meaning, _)| *meaning);
            let default = metadata
                .map(|(_, _, name)| name.to_string())
                .or_else(|| {
                    output.and_then(|table| {
                        table
                            .keys
                            .iter()
                            .position(|k| k == key)
                            .map(|i| output_column_default_label(table, i))
                    })
                })
                .unwrap_or_else(|| key.clone());
            ui.push_id(key, |ui| {
                ui.horizontal(|ui| {
                    if settings::move_button(ui, true, index > 0).clicked() {
                        movement = Some((index, index - 1));
                    }
                    if settings::move_button(ui, false, index + 1 < keys.len()).clicked() {
                        movement = Some((index, index + 1));
                    }
                    ui.add_sized(
                        [110.0, 24.0],
                        egui::Label::new(format!("{meaning}:")).halign(egui::Align::RIGHT),
                    )
                    .on_hover_text(key);
                    let mut label = config.labels.get(key).cloned().unwrap_or(default);
                    if ui
                        .add(settings::text_edit(ui, &mut label).desired_width(220.0))
                        .changed()
                    {
                        config.labels.insert(key.clone(), label);
                    }
                });
            });
        }
        if let Some((a, b)) = movement {
            keys.swap(a, b);
            config.order = keys;
        }
        if ui.button("Reset natural column order").clicked() {
            config.order.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 560.0),
                )),
                events,
                ..Default::default()
            },
            draw,
        );
        output.textures_delta.clear();
        output
    }

    pub(super) fn text_point(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => {
                    Some(text.pos + egui::vec2(3.0, 3.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing text {label}"))
    }

    pub(super) fn pointer(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn cage_paste_infers_headers_but_plate_input_has_no_headerless_option() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut state = Workflow::default();
        let mut has_header = true;
        let output = frame(&ctx, vec![], |ui| state.input_ui(ui, Some(&mut has_header)));
        for annotation in [
            "Headerless TSV - select columns in Config",
            "TSV with headers - copy cells from Excel",
        ] {
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == annotation)));
        }
        let paste = text_point(&output, "Paste / replace");
        for (text, expected) in [("A\t2\tF", false), ("Cage #\tQTY\tSEX\nA\t1\tF", true)] {
            frame(
                &ctx,
                vec![egui::Event::PointerMoved(paste), pointer(paste, true)],
                |ui| state.input_ui(ui, Some(&mut has_header)),
            );
            frame(&ctx, vec![pointer(paste, false)], |ui| {
                state.input_ui(ui, Some(&mut has_header))
            });
            frame(&ctx, vec![egui::Event::Paste(text.into())], |ui| {
                state.input_ui(ui, Some(&mut has_header))
            });
            assert_eq!(has_header, expected, "{text}");
            assert_eq!(state.input, text);
            assert_eq!(state.parsed.as_ref().unwrap().rows.len(), 1);
        }
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut state = Workflow::default();
        let output = frame(&ctx, vec![], |ui| state.input_ui(ui, None));
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Input includes header row")));
        let paste = text_point(&output, "Paste / replace");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(paste), pointer(paste, true)],
            |ui| state.input_ui(ui, None),
        );
        frame(&ctx, vec![pointer(paste, false)], |ui| {
            state.input_ui(ui, None)
        });
        frame(
            &ctx,
            vec![egui::Event::Paste("Cage\t#\tGene\nA (F)\t1\t+/+".into())],
            |ui| state.input_ui(ui, None),
        );
        assert!(state.parsed.as_ref().unwrap().has_header);
    }

    #[test]
    fn cage_output_columns_show_field_meanings_without_reset_or_confirmation_controls() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        ctx.style_mut_of(egui::Theme::Light, |style| style.animation_time = 0.0);
        let table = parse_tsv_with_header("Cage #\tQTY\tSEX\nA\t1\tF", true).unwrap();
        let mut config = config::CageToPlateConfig::default();
        let output = frame(&ctx, vec![], |ui| {
            cage_to_plate::settings_ui(ui, Some(&table), None, &mut config)
        });
        let section = text_point(&output, "Output columns");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(section), pointer(section, true)],
            |ui| cage_to_plate::settings_ui(ui, Some(&table), None, &mut config),
        );
        frame(&ctx, vec![pointer(section, false)], |ui| {
            cage_to_plate::settings_ui(ui, Some(&table), None, &mut config)
        });
        let output = frame(&ctx, vec![], |ui| {
            cage_to_plate::settings_ui(ui, Some(&table), None, &mut config)
        });
        for label in [
            "Cage ID:",
            "Within-cage ID:",
            "Ear label:",
            "Plate ID:",
            "Well ID:",
            "Global ID:",
            "Cage",
            "Sample",
            "Ear",
            "Plate",
            "Well",
            "#",
        ] {
            text_point(&output, label);
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Reset name")));
        assert!(
            config.output.labels.is_empty(),
            "rendering defaults must not insert overrides"
        );
        config.layout = config::CageLayout::SeparateColumn;
        let output = frame(&ctx, vec![], |ui| {
            cage_to_plate::settings_ui(ui, Some(&table), None, &mut config)
        });
        text_point(&output, "SEX");
    }

    #[test]
    fn output_column_defaults_include_all_hierarchical_header_levels() {
        let table = OutputTable {
            keys: vec!["genotype:example".into(), "cage".into()],
            headers: vec![
                vec!["5XFAD, +/+".into(), "".into()],
                vec!["LSL, +/+".into(), "Cage #".into()],
            ],
            rows: vec![],
        };
        assert_eq!(
            output_column_default_label(&table, 0),
            "5XFAD, +/+ | LSL, +/+"
        );
        assert_eq!(output_column_default_label(&table, 1), "Cage #");
    }

    #[test]
    fn both_copy_buttons_use_validated_cache_and_stale_gating() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut state = Workflow {
            input: "Cage #\tQTY\tSEX\nA\t1\tF".into(),
            ..Default::default()
        };
        state.reparse(true);
        let table = OutputTable {
            keys: vec!["cage".into(), "sample".into()],
            headers: vec![
                vec!["Outer".into(), "".into()],
                vec!["Cage".into(), "Sample".into()],
            ],
            rows: vec![vec!["A".into(), "1".into()]],
        };
        let draw =
            |ui: &mut egui::Ui, state: &mut Workflow| state.actions(ui, |_| Ok(table.clone()));
        let output = frame(&ctx, vec![], |ui| draw(ui, &mut state));
        let convert = text_point(&output, "Convert");
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(convert), pointer(convert, true)],
            |ui| draw(ui, &mut state),
        );
        let output = frame(&ctx, vec![pointer(convert, false)], |ui| {
            draw(ui, &mut state)
        });
        assert!(!state.stale);
        assert_eq!(state.output_tsv_without_headers, "A\t1\n");
        let buttons = [
            (
                text_point(&output, "Copy output (with headers)"),
                state.output_tsv.clone(),
            ),
            (
                text_point(&output, "Copy output (without headers)"),
                state.output_tsv_without_headers.clone(),
            ),
        ];
        for (pos, expected) in &buttons {
            frame(
                &ctx,
                vec![egui::Event::PointerMoved(*pos), pointer(*pos, true)],
                |ui| draw(ui, &mut state),
            );
            let output = frame(&ctx, vec![pointer(*pos, false)], |ui| draw(ui, &mut state));
            assert!(output.platform_output.commands.iter().any(
                |command| matches!(command, egui::OutputCommand::CopyText(text) if text == expected)
            ));
        }
        state.invalidate();
        for (pos, _) in &buttons {
            frame(
                &ctx,
                vec![egui::Event::PointerMoved(*pos), pointer(*pos, true)],
                |ui| draw(ui, &mut state),
            );
            let output = frame(&ctx, vec![pointer(*pos, false)], |ui| draw(ui, &mut state));
            assert!(
                !output
                    .platform_output
                    .commands
                    .iter()
                    .any(|command| matches!(command, egui::OutputCommand::CopyText(_)))
            );
        }
    }

    #[test]
    fn header_checkbox_reparses_immediately_and_preserves_pasted_text() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut state = Workflow {
            input: "Cage #\tQTY\tSEX\nA\t1\tF".into(),
            ..Default::default()
        };
        state.reparse(true);
        state.stale = false;
        let original = state.input.clone();
        let mut has_header = true;
        let render = |events, state: &mut Workflow, has_header: &mut bool| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(720.0, 560.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| state.input_ui(ui, Some(has_header)),
            );
            output.textures_delta.clear();
            output
        };
        let output = render(vec![], &mut state, &mut has_header);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Input includes header row" => {
                    Some(text.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .expect("visible checkbox label");
        let click = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        render(
            vec![egui::Event::PointerMoved(position), click(true)],
            &mut state,
            &mut has_header,
        );
        render(vec![click(false)], &mut state, &mut has_header);
        assert!(!has_header && state.stale);
        assert_eq!(state.input, original);
        let parsed = state.parsed.as_ref().unwrap();
        assert!(!parsed.has_header);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.source_rows, [1, 2]);
        render(vec![click(true)], &mut state, &mut has_header);
        render(vec![click(false)], &mut state, &mut has_header);
        assert!(has_header);
        assert_eq!(state.input, original);
        assert_eq!(state.parsed.as_ref().unwrap().rows, [vec!["A", "1", "F"]]);
    }
}
