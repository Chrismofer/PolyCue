mod color;
mod render;
mod io;
mod gui;

use eframe::{egui, NativeOptions};
use gui::AppState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 1200.0])
            .with_min_inner_size([800.0, 600.0])
            .with_position([100.0, 100.0])
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(
        "Poly Cue",
        native_options,
        Box::new(|cc| {
            let mut app = AppState::new();
            app.regenerate(&cc.egui_ctx);
            Box::new(app)
        }),
    )?;
    Ok(())
}
