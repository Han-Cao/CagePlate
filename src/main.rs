#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 850.0])
            .with_min_inner_size([720.0, 560.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "CagePlate",
        options,
        Box::new(|cc| Ok(Box::new(ui::CagePlateApp::new(&cc.egui_ctx)))),
    )
}
