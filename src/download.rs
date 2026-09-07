use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use futures_util::stream::StreamExt;
use crate::types::{IaGuiApp, AppMessage, IaItem};

impl IaGuiApp {
    pub fn execute_download(&mut self, item: IaItem, ctx: &eframe::egui::Context) {
        let id = item.identifier.clone();
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        let out_dir = self.download_directory.clone();

        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

            let mut metadata_url = match reqwest::Url::parse("https://archive.org") {
                Ok(url) => url,
                     Err(e) => { let _ = tx.send(AppMessage::Log(format!("URL Base Failure: {}", e))); return; }
            };

            if let Ok(mut path_segments) = metadata_url.path_segments_mut() {
                path_segments.push("metadata");
                path_segments.push(&id);
            }

            let res = match client.get(metadata_url).send().await {
                Ok(r) => r,
                     Err(e) => { let _ = tx.send(AppMessage::Log(format!("Metadata Access Blocked: {}", e))); return; }
            };

            let json_body = match res.text().await { Ok(t) => t, Err(_) => return };
            let parsed_json: serde_json::Value = match serde_json::from_str(&json_body) {
                Ok(v) => v,
                     Err(e) => { let _ = tx.send(AppMessage::Log(format!("Parse Mismatch: {}", e))); return; }
            };

            let files_array = match parsed_json["files"].as_array() {
                Some(arr) => arr,
                     None => { let _ = tx.send(AppMessage::Log("No inner files indices found.".to_string())); return; }
            };

            let mut valid_filenames = Vec::new();
            for file_entry in files_array {
                if let Some(name_str) = file_entry["name"].as_str() {
                    // Ignore junk sidecar structural cache formats
                    if name_str.ends_with(".xml") || name_str.ends_with(".sqlite") || name_str.ends_with(".json") {
                        continue;
                    }
                    valid_filenames.push(name_str.to_string());
                }
            }

            let total_files = valid_filenames.len();
            if total_files == 0 {
                let _ = tx.send(AppMessage::Log("No valid downloadable media assets found.".to_string()));
                return;
            }

            let _ = tx.send(AppMessage::DownloadStarted { total_files });

            let clean_folder_name = item.title.clone().unwrap_or_else(|| id.clone());
            let safe_folder_name = clean_folder_name.chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
            .collect::<String>();
            let target_item_folder = out_dir.join(safe_folder_name.trim());
            let _ = fs::create_dir_all(&target_item_folder);

            let completed_counter = Arc::new(AtomicUsize::new(0));
            let mut join_set = JoinSet::new();
            let client_shared = Arc::new(client);
            let id_shared = Arc::new(id.clone());
            let folder_shared = Arc::new(target_item_folder.clone());

            for file_name in valid_filenames {
                let t_client = Arc::clone(&client_shared);
                let t_id = Arc::clone(&id_shared);
                let t_folder = Arc::clone(&folder_shared);
                let t_counter = Arc::clone(&completed_counter);
                let t_tx = tx.clone();
                let t_ctx = ctx_clone.clone();
                let f_name_clone = file_name.clone();
                let t_token = token.clone();

                join_set.spawn(async move {
                    if t_token.is_cancelled() { return; }

                    let mut individual_dl_url = match reqwest::Url::parse("https://archive.org") {
                        Ok(url) => url,
                               Err(_) => return,
                    };

                    if let Ok(mut path_segments) = individual_dl_url.path_segments_mut() {
                        path_segments.push("download");
                        path_segments.push(&t_id);
                        path_segments.push(&file_name);
                    }

                    // Dynamically map subfolders (e.g., "Season 1/Episode 01.mp4") safely
                    let local_filepath = t_folder.join(&file_name);
                    if let Some(parent) = local_filepath.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Ok(file_res) = t_client.get(individual_dl_url).send().await {
                        if file_res.status().is_success() {
                            if let Ok(mut file) = File::create(&local_filepath) {
                                let mut stream = file_res.bytes_stream();
                                while let Some(chunk_result) = stream.next().await {
                                    if t_token.is_cancelled() { return; }

                                    if let Ok(chunk) = chunk_result {
                                        let bytes_count = chunk.len();
                                        if file.write_all(&chunk).is_ok() {
                                            let _ = t_tx.send(AppMessage::DownloadProgressUpdate {
                                                completed_files: t_counter.load(Ordering::Relaxed),
                                                              file_name: f_name_clone.clone(),
                                                              bytes_received: bytes_count,
                                            });
                                            t_ctx.request_repaint();
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let cur_completed = t_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    let _ = t_tx.send(AppMessage::DownloadProgressUpdate {
                        completed_files: cur_completed,
                        file_name: f_name_clone,
                        bytes_received: 0,
                    });
                    t_ctx.request_repaint();
                });
            }

            while let Some(_res) = join_set.join_next().await {
                if token.is_cancelled() {
                    join_set.shutdown().await;
                    let _ = tx.send(AppMessage::DownloadAborted);
                    ctx_clone.request_repaint();
                    return;
                }
            }

            let _ = tx.send(AppMessage::DownloadFinished(target_item_folder.to_string_lossy().to_string()));
            ctx_clone.request_repaint();
        });
    }

    pub fn abort_download(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.cancel();
        }
    }
}
