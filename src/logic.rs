use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use futures_util::stream::StreamExt;
use crate::types::{IaGuiApp, SearchFilter, AppMessage, IaSearchResponse, IaItem};

impl IaGuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let default_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|_| PathBuf::from("."));

        let history = Self::load_history_from_disk();

        Self {
            search_input: "nasa video".to_string(),
            filter: SearchFilter::All,
            download_directory: default_dir,
            status_text: "Ready".to_string(),
            logs: vec!["Better-IA Initialized successfully with tracking optimizations.".to_string()],
            results: Vec::new(),
            tx,
            rx,
            total_download_files: 0,
            completed_download_files: 0,
            current_file_label: String::new(),
            cancel_token: None,
            total_bytes_downloaded: 0,
            download_start_time: None,
            current_speed_mbps: 0.0,
            search_history: history,
        }
    }

    pub fn update_channels(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::Log(text) => {
                    self.logs.push(text.clone());
                    self.status_text = text;
                }
                AppMessage::SearchResults(items) => {
                    self.results = items;
                    self.status_text = format!("Found {} entries.", self.results.len());
                }
                AppMessage::DownloadStarted { total_files } => {
                    self.total_download_files = total_files;
                    self.completed_download_files = 0;
                    self.total_bytes_downloaded = 0;
                    self.current_speed_mbps = 0.0;
                    self.download_start_time = Some(Instant::now());
                    self.status_text = format!("Downloading collection queue (0/{})", total_files);
                }
                AppMessage::DownloadProgressUpdate { completed_files, file_name, bytes_received } => {
                    self.completed_download_files = completed_files;
                    self.current_file_label = file_name;
                    self.total_bytes_downloaded += bytes_received;

                    if let Some(start_time) = self.download_start_time {
                        let elapsed = start_time.elapsed().as_secs_f64();
                        if elapsed > 0.0 {
                            let bytes_per_sec = self.total_bytes_downloaded as f64 / elapsed;
                            self.current_speed_mbps = bytes_per_sec / (1024.0 * 1024.0);
                        }
                    }
                    self.status_text = format!("Downloading ({}/{}): {}", completed_files, self.total_download_files, self.current_file_label);
                }
                AppMessage::DownloadFinished(folder) => {
                    self.status_text = "All Downloads Complete!".to_string();
                    self.completed_download_files = self.total_download_files;
                    self.cancel_token = None;
                    self.logs.push(format!("Saved parallel collection inside: {}", folder));
                }
                AppMessage::DownloadAborted => {
                    self.status_text = "Download Canceled by User.".to_string();
                    self.total_download_files = 0;
                    self.completed_download_files = 0;
                    self.cancel_token = None;
                    self.logs.push("Active thread pools canceled immediately.".to_string());
                }
            }
        }
    }

    pub fn execute_search(&mut self, ctx: &eframe::egui::Context) {
        let raw_query = self.search_input.trim().to_string();
        let filter_type = self.filter;
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();

        if raw_query.is_empty() { return; }

        if !self.search_history.contains(&raw_query) {
            self.search_history.insert(0, raw_query.clone());
            if self.search_history.len() > 10 { self.search_history.pop(); }
            Self::save_history_to_disk(&self.search_history);
        }

        self.status_text = "Searching Catalog...".to_string();
        self.results.clear();

        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let query = if raw_query.contains(' ') && !raw_query.starts_with('"') {
                format!("\"{}\"", raw_query)
            } else {
                raw_query
            };

            let filter_query = match filter_type {
                SearchFilter::Collection => format!("{} AND mediatype:collection", query),
                SearchFilter::SingleFile => format!("{} AND NOT mediatype:collection", query),
                SearchFilter::All => query,
            };

            let mut api_url = match reqwest::Url::parse("https://archive.org") {
                Ok(url) => url,
                Err(e) => {
                    let _ = tx.send(AppMessage::Log(format!("URL Parse Failure: {}", e)));
                    return;
                }
            };

            if let Ok(mut path_segments) = api_url.path_segments_mut() {
                path_segments.push("advancedsearch.php");
            }

            api_url.query_pairs_mut().extend_pairs(&[
                ("q", filter_query.as_str()),
                ("fl[]", "identifier"),
                ("fl[]", "title"),
                ("fl[]", "mediatype"),
                ("rows", "25"),
                ("output", "json"),
            ]);

            match client.get(api_url).send().await {
                Ok(resp) => {
                    if let Ok(search_data) = resp.json::<IaSearchResponse>().await {
                        let _ = tx.send(AppMessage::SearchResults(search_data.response.docs));
                    }
                }
                Err(e) => { let _ = tx.send(AppMessage::Log(format!("Network Failure: {}", e))); }
            }
            ctx_clone.request_repaint();
        });
    }
    pub fn execute_download(&mut self, item: IaItem, ctx: &eframe::egui::Context) {
        let id = item.identifier.clone();
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        let out_dir = self.download_directory.clone();

        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());

        tokio::spawn(async move {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap_or_default();
            let mut metadata_url = match reqwest::Url::parse("https://archive.org") {
                Ok(url) => url,
                     Err(e) => { let _ = tx.send(AppMessage::Log(format!("URL Base Failure: {}", e))); return; }
            };

            if let Ok(mut path_segments) = metadata_url.path_segments_mut() {
                path_segments.push("metadata");
                path_segments.push(&id);
            }

            // FIX: Pass metadata_url by value instead of reference to satisfy IntoUrl trait
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
                    valid_filenames.push(name_str.to_string());
                }
            }

            let total_files = valid_filenames.len();
            if total_files == 0 { return; }

            let _ = tx.send(AppMessage::DownloadStarted { total_files });
            let target_item_folder = out_dir.join(&id);
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

                    let local_filepath = t_folder.join(&file_name);
                    if let Some(parent) = local_filepath.parent() { let _ = fs::create_dir_all(parent); }

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

    fn load_history_from_disk() -> Vec<String> {
        let path = PathBuf::from(".better_ia_history.json");
        if let Ok(file_content) = fs::read_to_string(path) {
            serde_json::from_str(&file_content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_history_to_disk(history: &Vec<String>) {
        let path = PathBuf::from(".better_ia_history.json");
        if let Ok(serialized) = serde_json::to_string(history) {
            let _ = fs::write(path, serialized);
        }
    }
}
