use eframe::egui;
use crate::types::{IaGuiApp, SearchFilter};

impl eframe::App for IaGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_channels();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📦 Better-IA: Internet Archive Explorer");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Search Catalog:");
                let text_res = ui.text_edit_singleline(&mut self.search_input);
                if text_res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.execute_search(ctx);
                }
            });
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Save Folder:");
                ui.label(self.download_directory.to_string_lossy().to_string());
                if ui.button("📁 Change Directory").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.download_directory = folder;
                    }
                }
            });
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Filter Type:");
                ui.radio_value(&mut self.filter, SearchFilter::All, "All Items");
                ui.radio_value(&mut self.filter, SearchFilter::Collection, "Collections");
                ui.radio_value(&mut self.filter, SearchFilter::SingleFile, "Single Files");
            });
            ui.add_space(10.0);

            if ui.button("🔍 Search Internet Archive").clicked() {
                self.execute_search(ctx);
            }

            ui.add_space(10.0);
            ui.label(format!("Status: {}", self.status_text));
            ui.add_space(5.0);

            // NATIVE VISUAL PROGRESS BAR ARCHITECTURE ROW
            if self.total_download_files > 0 {
                let progress_fraction = self.completed_download_files as f32 / self.total_download_files as f32;
                let bar_text = format!("{}% ({}/{})", (progress_fraction * 100.0) as usize, self.completed_download_files, self.total_download_files);
                
                ui.add(
                    egui::ProgressBar::new(progress_fraction)
                        .text(bar_text)
                        .animate(self.completed_download_files < self.total_download_files)
                );
                ui.add_space(5.0);
            }

            ui.separator();

            ui.label("Results:");
            egui::ScrollArea::vertical().max_height(160.0).id_salt("results_scroll").show(ui, |ui| {
                if self.results.is_empty() {
                    ui.label("No items found. Run an Archive search.");
                } else {
                    let results_clone = self.results.clone();
                    for item in results_clone {
                        ui.horizontal(|ui| {
                            let title = item.title.clone().unwrap_or_else(|| item.identifier.clone());
                            let mediatype = item.mediatype.clone().unwrap_or_else(|| "unknown".to_string());
                            ui.label(format!("[{}] {}", mediatype.to_uppercase(), title));
                            
                            if ui.button("📥 Download").clicked() {
                                self.execute_download(item, ctx);
                            }
                        });
                    }
                }
            });

            ui.separator();
            ui.label("Application Action Logs:");
            egui::ScrollArea::vertical().max_height(100.0).id_salt("logs_scroll").show(ui, |ui| {
                for log in &self.logs {
                    ui.label(log);
                }
            });
        });
    }
}

