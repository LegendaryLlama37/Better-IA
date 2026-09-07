use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use crate::types::{IaGuiApp, SearchFilter, AppMessage, IaSearchResponse};

impl IaGuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let default_dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(|_| PathBuf::from("."));

        let history = Self::load_history_from_disk();

        Self {
            search_input: "scooby-doo".to_string(),
            local_filter_input: String::new(),
            filter: SearchFilter::TvShow,
            download_directory: default_dir,
            status_text: "Ready".to_string(),
            logs: vec!["Better-IA Scaled Tracker Initialized Successfully.".to_string()],
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
                    self.status_text = format!("Found {} structured media entries.", self.results.len());
                }
                AppMessage::DownloadStarted { total_files } => {
                    self.total_download_files = total_files;
                    self.completed_download_files = 0;
                    self.total_bytes_downloaded = 0;
                    self.current_speed_mbps = 0.0;
                    self.download_start_time = Some(Instant::now());
                    self.status_text = format!("Downloading collection structure (0/{})", total_files);
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
                    self.status_text = "Structured Download Complete!".to_string();
                    self.completed_download_files = self.total_download_files;
                    self.cancel_token = None;
                    self.logs.push(format!("Saved collection tree inside: {}", folder));
                }
                AppMessage::DownloadAborted => {
                    self.status_text = "Download Canceled.".to_string();
                    self.total_download_files = 0;
                    self.completed_download_files = 0;
                    self.cancel_token = None;
                    self.logs.push("Active download worker pools aborted immediately.".to_string());
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

        self.status_text = "Filtering Catalog Index...".to_string();
        self.results.clear();

        tokio::spawn(async move {
            let client = reqwest::Client::new();

            // STRICT SCRAPING RULE: Enforce explicit title and identifier bounds to drop irrelevant files
            let target_field_query = format!("(title:({0}) OR identifier:({0}))", raw_query);

            let filter_query = match filter_type {
                SearchFilter::TvShow => format!("{} AND mediatype:movies AND collection:(*)", target_field_query),
                     SearchFilter::MusicAlbum => format!("{} AND mediatype:audio AND collection:(*)", target_field_query),
                     SearchFilter::BookSeries => format!("{} AND mediatype:texts AND collection:(*)", target_field_query),
                     SearchFilter::SingleFile => format!("{} AND NOT mediatype:collection", target_field_query),
                     SearchFilter::All => target_field_query,
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

            // Added "item_size" payload directive so we can parse and display size stats
            api_url.query_pairs_mut().extend_pairs(&[
                ("q", filter_query.as_str()),
                                                   ("fl[]", "identifier"),
                                                   ("fl[]", "title"),
                                                   ("fl[]", "mediatype"),
                                                   ("fl[]", "item_size"),
                                                   ("sort[]", "downloads desc"),
                                                   ("rows", "50"),
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

    pub(crate) fn load_history_from_disk() -> Vec<String> {
        let path = PathBuf::from(".better_ia_history.json");
        if let Ok(file_content) = fs::read_to_string(path) {
            serde_json::from_str(&file_content).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub(crate) fn save_history_to_disk(history: &Vec<String>) {
        let path = PathBuf::from(".better_ia_history.json");
        if let Ok(serialized) = serde_json::to_string(history) {
            let _ = fs::write(path, serialized);
        }
    }
}
