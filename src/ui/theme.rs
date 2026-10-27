//! Explicit light visuals and an opaque root background, independent of OS theme.

use eframe::egui;

pub(super) const SECTION_FONT: &str = "CagePlateBold";

pub(super) fn apply(ctx: &egui::Context) {
    // egui's RichText::strong only changes color. Use a real bold face for
    // headings while keeping the default fonts/fallbacks for body text/icons.
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        SECTION_FONT.into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Ubuntu-Bold.ttf")).into(),
    );
    let mut family = vec![SECTION_FONT.into()];
    family.extend(
        fonts
            .families
            .get(&egui::FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Name(SECTION_FONT.into()), family);
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(egui::Color32::from_gray(25));
    visuals.panel_fill = egui::Color32::from_gray(248);
    visuals.window_fill = egui::Color32::WHITE;
    visuals.extreme_bg_color = egui::Color32::WHITE;
    visuals.text_edit_bg_color = Some(egui::Color32::WHITE);
    visuals.warn_fg_color = egui::Color32::from_rgb(133, 77, 14);
    visuals.error_fg_color = egui::Color32::from_rgb(185, 28, 28);
    ctx.set_visuals(visuals);
}

pub(super) fn root_frame(style: &egui::Style) -> egui::Frame {
    // eframe 0.37 supplies a root Ui without a background. A transparent Frame
    // would expose the native clear color behind otherwise light widgets.
    egui::Frame::central_panel(style).inner_margin(12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: egui::Color32) -> f64 {
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
    }

    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn section_titles_use_a_bundled_bold_face_not_just_stronger_color() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            super::super::settings::section_title(ui, "Input");
        });
        output.textures_delta.clear();
        let title = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Input" => Some(text),
                _ => None,
            })
            .expect("section title");
        assert!(
            title
                .galley
                .job
                .sections
                .iter()
                .all(|section| section.format.font_id.family
                    == egui::FontFamily::Name(SECTION_FONT.into()))
        );
    }

    #[test]
    fn light_theme_stays_light_when_the_system_is_dark() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        apply(&ctx);
        let mut output = ctx.run_ui(
            egui::RawInput {
                system_theme: Some(egui::Theme::Dark),
                ..Default::default()
            },
            |ui| {
                let visuals = ui.visuals();
                assert!(!visuals.dark_mode);
                assert_eq!(visuals.extreme_bg_color, egui::Color32::WHITE);
                assert_eq!(visuals.text_edit_bg_color(), egui::Color32::WHITE);
                assert_eq!(root_frame(ui.style()).fill, visuals.panel_fill);
                assert_eq!(root_frame(ui.style()).fill.a(), 255);
                assert!(luminance(visuals.panel_fill) > 0.9);
            },
        );
        // No renderer is attached in this headless style test.
        output.textures_delta.clear();
    }

    #[test]
    fn text_errors_and_warnings_have_readable_light_theme_contrast() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let style = ctx.style_of(egui::Theme::Light);
        let visuals = &style.visuals;
        let text = visuals.override_text_color.expect("explicit foreground");
        for background in [
            visuals.panel_fill,
            visuals.window_fill,
            visuals.extreme_bg_color,
            visuals.selection.bg_fill,
            visuals.widgets.inactive.bg_fill,
            visuals.widgets.hovered.bg_fill,
            visuals.widgets.active.bg_fill,
        ] {
            assert!(
                contrast(text, background) >= 4.5,
                "Insufficient text contrast on {background:?}"
            );
        }
        for foreground in [visuals.error_fg_color, visuals.warn_fg_color] {
            assert!(
                contrast(foreground, visuals.panel_fill) >= 4.5,
                "Insufficient diagnostic contrast for {foreground:?}"
            );
        }
    }
}
