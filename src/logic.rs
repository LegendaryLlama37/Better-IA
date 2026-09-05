use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::task::JoinSet;
use crate::types::{IaGuiApp, SearchFilter, AppMessage, IaSearchResponse, IaItem};

impl IaGuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let default_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|_| PathBuf::from("."));

        Self {
            search_input: "nasa video".to_string(),
            filter: SearchFilter::All,
            download_directory: default_dir,
            status_text: "Ready".to_string(),
            logs: vec!["Better-IA Initialized successfully.".to_string()],
            results: Vec::new(),
            tx,
            rx,
            total_download_files: 0,
            completed_download_files: 0,
            current_file_label: String::new(),
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
                    self.logs.push(format!("Populated {} search results.", self.results.len()));
                }
                AppMessage::DownloadStarted { total_files } => {
                    self.total_download_files = total_files;
                    self.completed_download_files = 0;
                    self.status_text = format!("Downloading collection queue (0/{})", total_files);
                }
                AppMessage::DownloadProgressUpdate { completed_files, file_name } => {
                    self.completed_download_files = completed_files;
                    self.current_file_label = file_name;
                    self.status_text = format!("Downloading ({}/{}): {}", completed_files, self.total_download_files, self.current_file_label);
                }
                AppMessage::DownloadFinished(folder) => {
                    self.status_text = "All Downloads Complete!".to_string();
                    self.completed_download_files = self.total_download_files;
                    self.logs.push(format!("Saved parallel collection inside: {}", folder));
                }
            }
        }
    }

    pub fn execute_search(&mut self, ctx: &eframe::egui::Context) {
        let raw_query = self.search_input.trim().to_string();
        let filter_type = self.filter;
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();

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
                    if !resp.status().is_success() {
                        let _ = tx.send(AppMessage::Log(format!("Archive API error code: {}", resp.status())));
                        return;
                    }

                    match resp.json::<IaSearchResponse>().await {
                        Ok(search_data) => {
                            let _ = tx.send(AppMessage::SearchResults(search_data.response.docs));
                        }
                        Err(e) => {
                            let _ = tx.send(AppMessage::Log(format!("JSON Deserialization Error: Layout Mismatch. Detail: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Log(format!("Network Failure: Request dropped by host. Detail: {}", e)));
                }
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn execute_download(&self, item: IaItem, ctx: &eframe::egui::Context) {
        let id = item.identifier.clone();
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        let out_dir = self.download_directory.clone();

        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default();
                
            // FIX: Enforce explicit structural path segmentation builder to prevent string collapsing
            let mut metadata_url = match reqwest::Url::parse("https://archive.org") {
                Ok(url) => url,
                Err(e) => {
                    let _ = tx.send(AppMessage::Log(format!("URL Base Failure: {}", e)));
                    return;
                }
            };

            if let Ok(mut path_segments) = metadata_url.path_segments_mut() {
                path_segments.push("metadata");
                path_segments.push(&id);
            }
            
            let res = match client.get(metadata_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(AppMessage::Log(format!("Metadata Access Blocked: {}", e)));
                    return;
                }
            };

            let json_body = match res.text().await {
                Ok(t) => t,
                Err(_) => return,
            };

            let parsed_json: serde_json::Value = match serde_json::from_str(&json_body) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.send(AppMessage::Log(format!("Payload Parse Mismatch: {}", e)));
                    return;
                }
            };

            let files_value = &parsed_json["files"];
            let files_array = match files_value.as_array() {
                Some(arr) => arr,
                None => {
                    let _ = tx.send(AppMessage::Log("No indexable elements located inside collection payload.".to_string()));
                    return;
                }
            };

            let mut valid_filenames = Vec::new();
            for file_entry in files_array {
                if let Some(name_str) = file_entry["name"].as_str() {
                    valid_filenames.push(name_str.to_string());
                }
            }

            let total_files = valid_filenames.len();
            if total_files == 0 {
                let _ = tx.send(AppMessage::Log("Empty manifest data index tracker layer.".to_string()));
                return;
            }

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

                join_set.spawn(async move {
                    // FIX: Enforce explicit structure path building for individual items to prevent parsing errors
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
                    if let Some(parent) = local_filepath.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Ok(file_res) = t_client.get(individual_dl_url).send().await {
                        if file_res.status().is_success() {
                            if let Ok(data_bytes) = file_res.bytes().await {
if let Ok(mut f) = File::create(&local_filepath) {let _ = f.write_all(&data_bytes);}}}}let cur_completed = t_counter.fetch_add(1, Ordering::SeqCst) + 1;let _ = t_tx.send(AppMessage::DownloadProgressUpdate {completed_files: cur_completed,file_name: f_name_clone,});t_ctx.request_repaint();});}while join_set.join_next().await.is_some() {}let _ = tx.send(AppMessage::DownloadFinished(target_item_folder.to_string_lossy().to_string()));ctx_clone.request_repaint();});}}
