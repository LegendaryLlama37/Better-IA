use eframe::egui;
use crate::types::{IaGuiApp, SearchFilter};

impl eframe::App for IaGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_channels();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📦 Better-IA: Internet Archive Explorer");
            ui.add_space(10.0);

            let history_snapshot = self.search_history.clone();

            if !history_snapshot.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Recent Searches:");
                    // FIX: Swapped from_id_salt to from_id_source for compatibility
                    egui::ComboBox::from_id_source("history_box")
                    .selected_text(&self.search_input)
                    .show_ui(ui, |ui| {
                        for historic_item in &history_snapshot {
                            if ui.selectable_value(&mut self.search_input, historic_item.clone(), historic_item).clicked() {
                                self.execute_search(ctx);
                            }
                        }
                    });
                });
                ui.add_space(5.0);
            }

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

            if self.total_download_files > 0 {
                let progress_fraction = self.completed_download_files as f32 / self.total_download_files as f32;
                let bar_text = format!(
                    "{}% ({}/{}) - {:.2} MB/s",
                                       (progress_fraction * 100.0) as usize,
                                       self.completed_download_files,
                                       self.total_download_files,
                                       self.current_speed_mbps
                );

                ui.horizontal(|ui| {
                    let desired_width = ui.available_width() * 0.75;

                    ui.add_sized(
                        [desired_width, 20.0],
                        egui::ProgressBar::new(progress_fraction)
                        .text(bar_text)
                        .animate(self.completed_download_files < self.total_download_files)
                    );

                    if self.completed_download_files < self.total_download_files {
                        if ui.button("🛑 Cancel").clicked() {
                            self.abort_download();
                        }
                    }
                });
                ui.add_space(5.0);
            }

            ui.separator();

            ui.label("Results:");
            // FIX: Removed .id_salt() modifier call strings from ScrollArea configurations
            egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
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
            egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                for log in &self.logs {
                    ui.label(log);
                }
            });
        });
    }
}
