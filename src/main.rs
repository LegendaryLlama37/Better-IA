mod types;
mod logic;
mod view;

use types::IaGuiApp;

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([700.0, 520.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Better-IA Desktop Client",
        options,
        Box::new(|cc| Ok(Box::new(IaGuiApp::new(cc)))),
    )
}

