use serde::Deserialize;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

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
    DownloadProgressUpdate { completed_files: usize, file_name: String },
    DownloadFinished(String),
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
    pub(crate) total_download_files: usize,
    pub(crate) completed_download_files: usize,
    pub(crate) current_file_label: String,
}

