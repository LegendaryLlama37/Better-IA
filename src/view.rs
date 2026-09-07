use eframe::egui;
use crate::types::{IaGuiApp, SearchFilter};

// Helper utility to convert raw byte values into human-readable strings
fn format_bytes_to_readable(bytes: u64) -> String {
    if bytes == 0 { return "Unknown Size".to_string(); }
    let gigabytes = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gigabytes >= 1.0 {
        return format!("{:.2} GB", gigabytes);
    }
    let megabytes = bytes as f64 / (1024.0 * 1024.0);
    format!("{:.1} MB", megabytes)
}

impl eframe::App for IaGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_channels();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📺 Better-IA: Internet Archive Media Explorer");
            ui.add_space(10.0);

            let history_snapshot = self.search_history.clone();

            if !history_snapshot.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Recent Queries:");
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
                ui.label("Destination:");
                ui.label(self.download_directory.to_string_lossy().to_string());
                if ui.button("📁 Choose Folder").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.download_directory = folder;
                    }
                }
            });
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Target Filter:");
                ui.radio_value(&mut self.filter, SearchFilter::TvShow, "📺 TV/Cartoons");
                ui.radio_value(&mut self.filter, SearchFilter::MusicAlbum, "🎵 Music Albums");
                ui.radio_value(&mut self.filter, SearchFilter::BookSeries, "📚 Book Series");
                ui.radio_value(&mut self.filter, SearchFilter::SingleFile, "📄 Single Files");
                ui.radio_value(&mut self.filter, SearchFilter::All, "🌐 All");
            });
            ui.add_space(10.0);

            if ui.button("🔍 Index & Run Query").clicked() {
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
                        if ui.button("⏹ Abort").clicked() {
                            self.abort_download();
                        }
                    }
                });
                ui.add_space(5.0);
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("⚡ Filter Current Results:");
                ui.text_edit_singleline(&mut self.local_filter_input);
                if !self.local_filter_input.is_empty() {
                    if ui.button("❌ Clear").clicked() {
                        self.local_filter_input.clear();
                    }
                }
            });
            ui.add_space(5.0);

            ui.label("Discovered Collections (Sorted by Popularity):");
            ui.add_space(2.0);

            egui::ScrollArea::vertical().id_source("collections_scroll").max_height(280.0).show(ui, |ui| {
                if self.results.is_empty() {
                    ui.label("No structured matches found. Try searching above.");
                } else {
                    let needle = self.local_filter_input.trim().to_lowercase();
                    let mut matched_count = 0;
                    let items_snapshot = self.results.clone();

                    for item in items_snapshot {
                        let title = item.title.clone().unwrap_or_else(|| item.identifier.clone());

                        if !needle.is_empty() && !title.to_lowercase().contains(&needle) {
                            continue;
                        }
                        matched_count += 1;

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            let mediatype = item.mediatype.clone().unwrap_or_else(|| "Item".to_string()).to_uppercase();
                            let size_bytes = item.item_size.unwrap_or(0);
                            let size_label = format_bytes_to_readable(size_bytes);

                            ui.colored_label(egui::Color32::from_rgb(100, 180, 240), format!("[{}]", mediatype));
                            ui.colored_label(egui::Color32::from_rgb(180, 220, 100), format!("[{}]", size_label));

                            // FIXED: Swapped out .bold() for compatible .strong() primitive modifier syntax
                            ui.label(egui::RichText::new(title).strong());

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗂️ Download").clicked() {
                                    self.execute_download(item, ctx);
                                }
                            });
                        });

                        ui.add_space(4.0);
                        ui.separator();
                    }

                    if matched_count == 0 && !needle.is_empty() {
                        ui.label("No results match your local sub-filter keyword.");
                    }
                }
            });

            ui.separator();
            ui.label("Logs:");
            egui::ScrollArea::vertical().id_source("logs_scroll").max_height(80.0).show(ui, |ui| {
                for log in &self.logs {
                    ui.label(log);
                }
            });
        });
    }
}
