mod types;
mod search;
mod download;
mod view;

use types::IaGuiApp;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // 1. INCREASE WINDOW SIZE: Expanded from [700, 520] to [950, 700] for comfortable reading
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
        .with_inner_size([950.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Better-IA Desktop Client",
        options,
        Box::new(|cc| {
            // 2. INCREASE UI TEXT/BUTTON SCALE: Zooms everything up cleanly by 15% (1.15)
            // Change 1.15 to 1.25 if you want it even bigger!
            cc.egui_ctx.set_zoom_factor(1.15);

            Ok(Box::new(IaGuiApp::new(cc)))
        }),
    )
}
