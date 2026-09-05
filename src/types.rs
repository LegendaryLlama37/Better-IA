use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Deserialize, Debug, Clone)]
pub struct IaSearchResponse {
    pub response: IaResponseData,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IaResponseData {
    pub docs: Vec<IaItem>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IaItem {
    pub identifier: String,
    pub title: Option<String>,
    pub mediatype: Option<String>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum SearchFilter {
    All,
    Collection,
    SingleFile,
}

pub enum AppMessage {
    Log(String),
    SearchResults(Vec<IaItem>),
    DownloadStarted { total_files: usize },
    DownloadProgressUpdate { completed_files: usize, file_name: String, bytes_received: usize },
    DownloadFinished(String),
    DownloadAborted,
}

pub struct IaGuiApp {
    pub(crate) search_input: String,
    pub(crate) filter: SearchFilter,
    pub(crate) download_directory: PathBuf,
    pub(crate) status_text: String,
    pub(crate) logs: Vec<String>,
    pub(crate) results: Vec<IaItem>,
    pub(crate) tx: Sender<AppMessage>,
    pub(crate) rx: Receiver<AppMessage>,
    
    // Advanced features state variables
    pub(crate) total_download_files: usize,
    pub(crate) completed_download_files: usize,
    pub(crate) current_file_label: String,
    pub(crate) cancel_token: Option<CancellationToken>,
    pub(crate) total_bytes_downloaded: usize,
    pub(crate) download_start_time: Option<std::time::Instant>,
    pub(crate) current_speed_mbps: f64,
    pub(crate) search_history: Vec<String>,
}

