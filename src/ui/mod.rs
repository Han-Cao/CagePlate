//! Native clipboard UI. Conversion logic remains in the shared library.

mod cage_to_plate;
mod plate_to_cage;
mod settings;
mod theme;

use ::cageplate::{
    Result,
    config::{self, AppConfig, OutputConfig},
    model::{InputTable, OutputTable},
    tsv::{parse_tsv, write_tsv},
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
    conversion_error: Option<String>,
    stale: bool,
}

impl Workflow {
    fn invalidate(&mut self) {
        self.stale = true;
        self.conversion_error = None;
    }

    fn input_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("1. Paste spreadsheet input");
        let mut replace = false;
        let mut clear = false;
        ui.horizontal(|ui| {
            replace = ui.button("Paste / replace").on_hover_text("Replace input with clipboard text. You can also focus the box and press Ctrl+V.").clicked();
            clear = ui.button("Clear").clicked();
            ui.label("Headered TSV · copy cells directly from Excel");
        });
        if replace || clear {
            self.input.clear();
        }
        let response = ui.add(
            egui::TextEdit::multiline(&mut self.input)
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
        if response.changed() || replace || clear {
            self.invalidate();
            match parse_tsv(&self.input) {
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
                .and_then(|output| Ok((write_tsv(&output)?, output)))
            {
                Ok((text, output)) => {
                    self.output_tsv = text;
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
        if ui
            .add_enabled(
                self.output.is_some() && !self.stale,
                egui::Button::new("Copy full output (with headers)"),
            )
            .clicked()
        {
            ui.ctx().copy_text(self.output_tsv.clone());
        }
        if let Some(error) = &self.conversion_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
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
    ui.heading("Output preview (read-only)");
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
    status: String,
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
            status: String::new(),
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
        theme::root_frame(ui.style()).show(ui, |ui| {
            let mut save = false;
            let mut restore = false;
            ui.horizontal_wrapped(|ui| {
                ui.heading("CagePlate");
                ui.selectable_value(&mut self.tab, 0, "Cage → Plate");
                ui.selectable_value(&mut self.tab, 1, "Plate → Cage");
                save = ui
                    .add_enabled(
                        self.config_path.is_some(),
                        egui::Button::new("Save as default"),
                    )
                    .clicked();
                restore = ui.button("Restore built-ins").clicked();
            });
            if restore {
                self.config = AppConfig::default();
                self.cage.invalidate();
                self.plate.invalidate();
                self.status = "Built-ins restored in memory. Save as default to persist.".into();
            }
            if let Some(path) = &self.config_path {
                ui.small(format!("Defaults: {}", path.display()));
            }
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
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt(("workflow", self.tab))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.push_id(self.tab, |ui| {
                        if self.tab == 0 {
                            cage_to_plate::show(ui, &mut self.cage, &mut self.config.cage_to_plate);
                        } else {
                            plate_to_cage::show(
                                ui,
                                &mut self.plate,
                                &mut self.config.plate_to_cage,
                            );
                        }
                    });
                });
            if save && let Some(path) = &self.config_path {
                self.status = match config::save_defaults(path, &self.config) {
                    Ok(()) => "Defaults saved. No input/output data was saved.".into(),
                    Err(error) => format!("Could not save defaults: {error}"),
                };
            }
        });
    }
}

// Output controls work even before a conversion for the fixed metadata columns.
fn output_settings(
    ui: &mut egui::Ui,
    config: &mut OutputConfig,
    output: Option<&OutputTable>,
    base: &[(&str, &str)],
) {
    ui.collapsing("Output column names and order", |ui| {
        ui.label("Rename final header cells; use arrows to move whole columns with their full header paths. Convert to discover genotype columns.");
        let mut keys: Vec<String> = output.map_or_else(|| base.iter().map(|(k, _)| k.to_string()).collect(), |t| t.keys.clone());
        let natural = keys.clone();
        keys.sort_by_key(|key| config.order.iter().position(|k| k == key).unwrap_or(config.order.len() + natural.iter().position(|k| k == key).unwrap_or(0)));
        let mut movement = None;
        for (index, key) in keys.iter().enumerate() {
            let default = output.and_then(|t| t.keys.iter().position(|k| k == key).map(|i| t.headers.last().expect("headers")[i].clone())).unwrap_or_else(|| base.iter().find(|(k, _)| *k == key).map_or(key.as_str(), |(_, label)| label).to_string());
            ui.push_id(key, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() { movement = Some((index, index - 1)); }
                    if ui.add_enabled(index + 1 < keys.len(), egui::Button::new("↓")).clicked() { movement = Some((index, index + 1)); }
                    let mut label = config.labels.get(key).cloned().unwrap_or(default);
                    if ui.add(egui::TextEdit::singleline(&mut label).desired_width(220.0)).changed() { config.labels.insert(key.clone(), label); }
                    if ui.small_button("Reset name").clicked() { config.labels.remove(key); }
                    if let Some(table) = output && let Some(i) = table.keys.iter().position(|k| k == key) { ui.small(table.headers.iter().map(|h| h[i].as_str()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" → ")).on_hover_text(key); }
                });
            });
        }
        if let Some((a, b)) = movement { keys.swap(a, b); config.order = keys; }
        if ui.button("Reset natural column order").clicked() { config.order.clear(); }
    });
}
