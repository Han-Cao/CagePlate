//! Workflow-local drafts. Only successful Apply/save actions change live state.

use super::{Workflow, cage_to_plate, plate_to_cage, settings};
use ::cageplate::{
    Error, Result,
    config::{self, AppConfig, CageToPlateConfig, PlateToCageConfig, WorkflowDefaults},
    model::InputTable,
    tsv::parse_tsv_with_header,
};
use eframe::egui;
use std::path::Path;

#[derive(Clone)]
enum Draft {
    Cage(Box<CageToPlateConfig>),
    Plate(Box<PlateToCageConfig>),
}

impl Draft {
    fn has_header(&self) -> bool {
        match self {
            Self::Cage(config) => config.has_header,
            Self::Plate(config) => config.has_header,
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Cage(config) => config.validate(),
            Self::Plate(config) => config.validate(),
        }
    }

    fn defaults(&self) -> WorkflowDefaults<'_> {
        match self {
            Self::Cage(config) => WorkflowDefaults::CageToPlate(config),
            Self::Plate(config) => WorkflowDefaults::PlateToCage(config),
        }
    }

    fn commit(&self, active: &mut AppConfig, cage: &mut Workflow, plate: &mut Workflow) {
        match self {
            Self::Cage(config) if active.cage_to_plate != **config => {
                let reparse = active.cage_to_plate.has_header != config.has_header;
                active.cage_to_plate = (**config).clone();
                cage.invalidate();
                if reparse {
                    cage.reparse(config.has_header);
                }
            }
            Self::Plate(config) if active.plate_to_cage != **config => {
                let reparse = active.plate_to_cage.has_header != config.has_header;
                active.plate_to_cage = (**config).clone();
                plate.invalidate();
                if reparse {
                    plate.reparse(config.has_header);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Action {
    Apply,
    Cancel,
    Reset,
    Save,
}

pub(super) struct ConfigDialog {
    draft: Draft,
    error: Option<String>,
    message: Option<String>,
    // Only needed when Reset gives the draft a different source-header mode.
    preview_mode: Option<bool>,
    preview_table: Option<InputTable>,
    preview_error: Option<String>,
}

impl ConfigDialog {
    pub(super) fn new(active: &AppConfig, tab: usize) -> Self {
        Self {
            draft: if tab == 0 {
                Draft::Cage(Box::new(active.cage_to_plate.clone()))
            } else {
                Draft::Plate(Box::new(active.plate_to_cage.clone()))
            },
            error: None,
            message: None,
            preview_mode: None,
            preview_table: None,
            preview_error: None,
        }
    }

    pub(super) fn is_cage(&self) -> bool {
        matches!(self.draft, Draft::Cage(_))
    }

    pub(super) fn show(
        &mut self,
        ctx: &egui::Context,
        state: &Workflow,
        active_has_header: bool,
        path: Option<&Path>,
    ) -> Option<Action> {
        let has_header = self.draft.has_header();
        if has_header != active_has_header && self.preview_mode != Some(has_header) {
            self.preview_mode = Some(has_header);
            match parse_tsv_with_header(&state.input, has_header) {
                Ok(table) => {
                    self.preview_table = Some(table);
                    self.preview_error = None;
                }
                Err(error) => {
                    self.preview_table = None;
                    self.preview_error =
                        (!state.input.trim().is_empty()).then(|| error.to_string());
                }
            }
        }
        let (table, parse_error) = if has_header == active_has_header {
            (state.parsed.as_ref(), state.input_error.as_ref())
        } else {
            (self.preview_table.as_ref(), self.preview_error.as_ref())
        };
        let is_cage = self.is_cage();
        let mut action = None;
        // Keep the white text editors distinct from their surrounding canvas,
        // just like the main input editor on the app's light-gray background.
        let style = ctx.global_style();
        let frame = egui::Frame::popup(&style).fill(style.visuals.panel_fill);
        let modal = egui::Modal::new(egui::Id::unique(("workflow_config", is_cage)))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 64.0).clamp(280.0, 850.0));
                settings::section_title(
                    ui,
                    if is_cage {
                        "Cage to Plate - Config"
                    } else {
                        "Plate to Cage - Config"
                    },
                );
                ui.small(if has_header {
                    "Input includes a header row. Map columns by name."
                } else {
                    "Headerless input. Select Column 1, Column 2, etc. from the source lists."
                });
                if let Some(error) = parse_error {
                    ui.colored_label(ui.visuals().warn_fg_color, error);
                }
                egui::ScrollArea::both()
                    .id_salt(("config_body", is_cage))
                    .max_height((ctx.content_rect().height() - 260.0).max(100.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| match &mut self.draft {
                        Draft::Cage(config) => {
                            cage_to_plate::settings_ui(ui, table, state.output.as_ref(), config)
                        }
                        Draft::Plate(config) => {
                            plate_to_cage::settings_ui(ui, table, state.output.as_ref(), config)
                        }
                    });
                ui.separator();
                if let Some(path) = path {
                    ui.small(format!("User defaults: {}", path.display()));
                } else {
                    ui.small("User defaults path unavailable; Apply still works.");
                }
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                if let Some(message) = &self.message {
                    ui.label(message);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Apply").clicked() {
                        action = Some(Action::Apply);
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(Action::Cancel);
                    }
                    if ui
                        .add_enabled(path.is_some(), egui::Button::new("Save as default"))
                        .on_hover_text(
                            "Save and apply only this workflow. No input/output data is saved.",
                        )
                        .clicked()
                    {
                        action = Some(Action::Save);
                    }
                    if ui
                        .button("Reset")
                        .on_hover_text("Stage this tool's built-in settings; Apply to activate.")
                        .clicked()
                    {
                        action = Some(Action::Reset);
                    }
                });
            });
        if modal.should_close() && action.is_none() {
            action = Some(Action::Cancel);
        }
        action
    }

    // Returns true only when the dialog should be discarded. The active config
    // itself is the Apply/save baseline; Cancel never rolls back committed work.
    pub(super) fn handle_action(
        &mut self,
        action: Action,
        active: &mut AppConfig,
        cage: &mut Workflow,
        plate: &mut Workflow,
        path: Option<&Path>,
    ) -> bool {
        self.error = None;
        self.message = None;
        match action {
            Action::Cancel => return true,
            Action::Reset => {
                self.draft = if self.is_cage() {
                    Draft::Cage(Box::default())
                } else {
                    Draft::Plate(Box::default())
                };
                self.message = Some(
                    "Tool defaults staged. Apply to activate; Save as default to persist.".into(),
                );
                return false;
            }
            Action::Apply | Action::Save => {}
        }
        let result = self.draft.validate().and_then(|()| {
            if matches!(action, Action::Save) {
                let path = path.ok_or_else(|| Error::new("User defaults path unavailable"))?;
                config::save_workflow_defaults(path, self.draft.defaults())?;
            }
            Ok(())
        });
        if let Err(error) = result {
            self.error = Some(error.to_string());
            return false;
        }
        self.draft.commit(active, cage, plate);
        if matches!(action, Action::Save) {
            self.message = Some(
                "This workflow's user defaults saved and applied. No input/output data was saved."
                    .into(),
            );
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::cageplate::{cage_to_plate::convert, config::read_config, tsv::write_tsv};

    fn converted() -> Workflow {
        let mut state = Workflow {
            input: "Cage #\tQTY\tSEX\nA\t1\tF".into(),
            ..Default::default()
        };
        state.reparse(true);
        let output = convert(
            state.parsed.as_ref().unwrap(),
            &CageToPlateConfig::default(),
        )
        .unwrap();
        state.output_tsv = write_tsv(&output).unwrap();
        state.output_tsv_without_headers =
            ::cageplate::tsv::write_tsv_without_headers(&output).unwrap();
        state.output = Some(output);
        state.stale = false;
        state
    }

    #[test]
    fn cancel_and_draft_reset_leave_live_config_and_copyable_output_untouched() {
        let mut active = AppConfig::default();
        active.cage_to_plate.plate.start_mouse = 42;
        let original = active.clone();
        let mut cage = converted();
        let output = cage.output.clone();
        let mut plate = Workflow::default();
        let mut dialog = ConfigDialog::new(&active, 0);
        assert!(!dialog.handle_action(Action::Reset, &mut active, &mut cage, &mut plate, None));
        assert_eq!(active, original);
        assert!(!cage.stale);
        assert_eq!(cage.output, output);
        assert!(dialog.handle_action(Action::Cancel, &mut active, &mut cage, &mut plate, None));
        assert_eq!(active, original);
        assert!(!cage.stale);
    }

    #[test]
    fn apply_is_workflow_local_and_unchanged_apply_keeps_copy_enabled() {
        let mut active = AppConfig::default();
        let mut cage = converted();
        let mut plate = converted();
        let mut dialog = ConfigDialog::new(&active, 0);
        assert!(dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        assert!(!cage.stale && !plate.stale);
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.plate.start_mouse = 42;
        assert!(!cage.stale);
        assert!(dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        assert_eq!(active.cage_to_plate.plate.start_mouse, 42);
        assert!(cage.stale && !plate.stale);
        cage.stale = false;
        let mut dialog = ConfigDialog::new(&active, 1);
        let Draft::Plate(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.mouse_label = "{ear_label}/{global_id}".into();
        assert!(dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        assert!(!cage.stale && plate.stale);
        assert_eq!(active.cage_to_plate.plate.start_mouse, 42);
    }

    #[test]
    fn validation_and_save_failures_leave_live_state_and_draft_intact() {
        let mut active = AppConfig::default();
        let original = active.clone();
        let mut cage = converted();
        let mut plate = converted();
        let mut dialog = ConfigDialog::new(&active, 0);
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.plate.rows = 0;
        assert!(!dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        assert!(dialog.error.is_some());
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.plate.rows = 8;
        config.plate.start_mouse = 42;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("defaults.json");
        std::fs::write(&path, "corrupt").unwrap();
        assert!(!dialog.handle_action(
            Action::Save,
            &mut active,
            &mut cage,
            &mut plate,
            Some(&path)
        ));
        assert!(dialog.error.is_some());
        assert_eq!(active, original);
        assert!(!cage.stale && !plate.stale);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "corrupt");
        let Draft::Cage(config) = &dialog.draft else {
            unreachable!()
        };
        assert_eq!(config.plate.start_mouse, 42);
    }

    #[test]
    fn save_applies_and_cancel_only_discards_later_edits() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("defaults.json");
        let mut active = AppConfig::default();
        active.plate_to_cage.mouse_label = "unsaved {ear_label}".into();
        let mut cage = converted();
        let mut plate = converted();
        let mut dialog = ConfigDialog::new(&active, 0);
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.plate.start_mouse = 42;
        assert!(!dialog.handle_action(
            Action::Save,
            &mut active,
            &mut cage,
            &mut plate,
            Some(&path)
        ));
        assert!(dialog.error.is_none());
        assert_eq!(active.cage_to_plate.plate.start_mouse, 42);
        let saved = read_config(&path).unwrap();
        assert_eq!(saved.cage_to_plate, active.cage_to_plate);
        assert_eq!(saved.plate_to_cage, PlateToCageConfig::default());
        assert_eq!(active.plate_to_cage.mouse_label, "unsaved {ear_label}");
        assert!(!plate.stale);
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.plate.start_mouse = 99;
        assert!(dialog.handle_action(
            Action::Cancel,
            &mut active,
            &mut cage,
            &mut plate,
            Some(&path)
        ));
        assert_eq!(active.cage_to_plate, saved.cage_to_plate);
        assert_eq!(read_config(&path).unwrap(), saved);
    }

    #[test]
    fn applying_header_mode_reparses_only_its_input_without_changing_text() {
        let mut active = AppConfig::default();
        let mut cage = converted();
        let mut plate = converted();
        cage.input = "A\t1\tF".into();
        cage.reparse(true);
        assert!(cage.parsed.is_none());
        let text = cage.input.clone();
        let mut dialog = ConfigDialog::new(&active, 0);
        let Draft::Cage(config) = &mut dialog.draft else {
            unreachable!()
        };
        config.has_header = false;
        assert!(dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        let parsed = cage.parsed.as_ref().unwrap();
        assert!(!parsed.has_header);
        assert_eq!(parsed.rows, [vec!["A", "1", "F"]]);
        assert_eq!(parsed.source_rows, [1]);
        assert_eq!(cage.input, text);
        assert!(cage.stale && !plate.stale);
        assert!(cage.input_error.is_none());
        let mut dialog = ConfigDialog::new(&active, 0);
        assert!(!dialog.handle_action(Action::Reset, &mut active, &mut cage, &mut plate, None));
        // Reset's alternative parse is draft-only, even when the tool default
        // expects a header and this single-record paste would become invalid.
        let ctx = egui::Context::default();
        super::super::theme::apply(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            dialog.show(ui.ctx(), &cage, false, None);
        });
        output.textures_delta.clear();
        assert_eq!(dialog.preview_mode, Some(true));
        assert!(dialog.preview_table.is_none());
        assert_eq!(cage.parsed.as_ref().unwrap().rows.len(), 1);
        assert!(!active.cage_to_plate.has_header);
        assert!(dialog.handle_action(Action::Apply, &mut active, &mut cage, &mut plate, None));
        assert!(active.cage_to_plate.has_header);
        assert!(cage.parsed.is_none());
        assert!(cage.input_error.is_some());
        assert_eq!(cage.input, text);
    }

    fn modal_frame(
        ctx: &egui::Context,
        dialog: &mut ConfigDialog,
        state: &Workflow,
        has_header: bool,
        events: Vec<egui::Event>,
    ) -> (Option<Action>, Vec<egui::epaint::ClippedShape>) {
        let mut action = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720.0, 560.0),
                )),
                events,
                ..Default::default()
            },
            |ui| action = dialog.show(ui.ctx(), state, has_header, None),
        );
        output.textures_delta.clear();
        (action, output.shapes)
    }

    #[test]
    fn both_config_modals_distinguish_white_text_editors_from_their_background() {
        use super::super::{tests::pointer, tests::text_point, theme};

        fn collect_rects<'a>(shape: &'a egui::Shape, rects: &mut Vec<&'a egui::epaint::RectShape>) {
            match shape {
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect_rects(shape, rects);
                    }
                }
                egui::Shape::Rect(rect) => rects.push(rect),
                _ => {}
            }
        }

        for tab in 0..=1 {
            let ctx = egui::Context::default();
            theme::apply(&ctx);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            let mut active = AppConfig::default();
            active.cage_to_plate.ears = vec!["R".into(), "L".into()];
            let mut state = converted();
            if tab == 1 {
                state.input = "Cage\tSample\tEar\tPlate\tWell\t#\nA (F)\t1\tR\t1\t1A\t1".into();
                state.reparse(true);
            }
            let mut dialog = ConfigDialog::new(&active, tab);
            let mut render = |events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1200.0, 1600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        dialog.show(ui.ctx(), &state, true, None);
                    },
                );
                output.textures_delta.clear();
                output
            };
            render(vec![]);
            let mut output = render(vec![]);
            for section in [
                if tab == 0 {
                    "Ear labels"
                } else {
                    "Mouse labeling"
                },
                "Output columns",
            ] {
                let position = text_point(&output, section);
                render(vec![
                    egui::Event::PointerMoved(position),
                    pointer(position, true),
                ]);
                render(vec![pointer(position, false)]);
                output = render(vec![]);
            }
            let title = text_point(
                &output,
                if tab == 0 {
                    "Cage to Plate - Config"
                } else {
                    "Plate to Cage - Config"
                },
            );
            let mut rects = vec![];
            for shape in &output.shapes {
                collect_rects(&shape.shape, &mut rects);
            }
            let background = ctx.global_style().visuals.panel_fill;
            let modal = rects
                .iter()
                .find(|rect| rect.rect.contains(title) && rect.fill == background)
                .expect("Config modal must use the app's light-gray background");
            assert_ne!(modal.fill, egui::Color32::WHITE);
            let editor_values = if tab == 0 {
                ["Cage", "R\nL", "Sample"]
            } else {
                ["Cage #", "F\nM", "{ear_label}{global_id}"]
            };
            for value in editor_values {
                let position = text_point(&output, value);
                assert!(
                    rects.iter().any(|rect| {
                        rect.fill == egui::Color32::WHITE
                            && rect.rect.contains(position)
                            && modal.rect.contains_rect(rect.rect)
                    }),
                    "editor {value:?} must have a white fill inside the gray modal"
                );
            }
        }
    }

    #[test]
    fn both_long_modals_keep_action_footers_visible_at_minimum_size_and_high_dpi() {
        for tab in 0..=1 {
            for scale in [1.0, 2.0] {
                let ctx = egui::Context::default();
                super::super::theme::apply(&ctx);
                ctx.set_pixels_per_point(scale);
                let mut active = AppConfig::default();
                active.cage_to_plate.has_header = false;
                active.plate_to_cage.has_header = true;
                active.plate_to_cage.groups = (1..=8)
                    .map(|column| config::GroupConfig {
                        column: config::ColumnRef::Position { column },
                        ..Default::default()
                    })
                    .collect();
                let mut state = converted();
                let has_header = tab == 1;
                state.reparse(has_header);
                let mut dialog = ConfigDialog::new(&active, tab);
                for _ in 0..2 {
                    modal_frame(&ctx, &mut dialog, &state, has_header, vec![]);
                }
                let (_, shapes) = modal_frame(&ctx, &mut dialog, &state, has_header, vec![]);
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 560.0));
                for label in ["Apply", "Cancel", "Save as default", "Reset"] {
                    let (clip, text) = shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == label => {
                                Some((shape.clip_rect, text))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| {
                            panic!("missing footer {label}, tab {tab}, scale {scale}")
                        });
                    let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                    assert!(
                        screen.contains_rect(rect) && clip.contains_rect(rect),
                        "footer {label} clipped at tab {tab}, scale {scale}: {rect:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn backdrop_cancels_but_escape_first_dismisses_an_open_combo_box() {
        let ctx = egui::Context::default();
        super::super::theme::apply(&ctx);
        let active = AppConfig::default();
        let state = converted();
        let mut dialog = ConfigDialog::new(&active, 1);
        for _ in 0..2 {
            modal_frame(&ctx, &mut dialog, &state, true, vec![]);
        }
        let (_, shapes) = modal_frame(&ctx, &mut dialog, &state, true, vec![]);
        let position = shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Sex on a new line" => {
                    Some(text.pos + egui::vec2(4.0, 4.0))
                }
                _ => None,
            })
            .expect("visible input layout combo box");
        let pointer = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        modal_frame(
            &ctx,
            &mut dialog,
            &state,
            true,
            vec![egui::Event::PointerMoved(position), pointer(position, true)],
        );
        modal_frame(
            &ctx,
            &mut dialog,
            &state,
            true,
            vec![pointer(position, false)],
        );
        let escape = |pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        assert!(
            modal_frame(&ctx, &mut dialog, &state, true, vec![escape(true)])
                .0
                .is_none()
        );
        modal_frame(&ctx, &mut dialog, &state, true, vec![escape(false)]);
        assert!(matches!(
            modal_frame(&ctx, &mut dialog, &state, true, vec![escape(true)]).0,
            Some(Action::Cancel)
        ));
        modal_frame(&ctx, &mut dialog, &state, true, vec![escape(false)]);
        let outside = egui::pos2(2.0, 2.0);
        modal_frame(
            &ctx,
            &mut dialog,
            &state,
            true,
            vec![egui::Event::PointerMoved(outside), pointer(outside, true)],
        );
        assert!(matches!(
            modal_frame(
                &ctx,
                &mut dialog,
                &state,
                true,
                vec![pointer(outside, false)]
            )
            .0,
            Some(Action::Cancel)
        ));
    }

    #[test]
    fn modal_renders_at_minimum_window_size_and_escape_requests_cancel() {
        let ctx = egui::Context::default();
        super::super::theme::apply(&ctx);
        let active = AppConfig::default();
        let mut dialog = ConfigDialog::new(&active, 0);
        let state = converted();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 560.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input.clone(), |ui| {
            assert!(dialog.show(ui.ctx(), &state, true, None).is_none());
        });
        output.textures_delta.clear();
        let mut input = input;
        input.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut output = ctx.run_ui(input, |ui| {
            assert!(matches!(
                dialog.show(ui.ctx(), &state, true, None),
                Some(Action::Cancel)
            ));
        });
        output.textures_delta.clear();
    }
}
