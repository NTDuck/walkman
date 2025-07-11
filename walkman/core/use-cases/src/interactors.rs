use async_trait::async_trait;
use triomphe::Arc;

use crate::{boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistProgressSnapshot, DownloadPlaylistRequestModel, DownloadVideoOutputBoundary}, gateways::Downloader};

pub struct DownloadPlaylistInteractor {
    download_playlist_output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,
    download_video_output_boundary: Arc<dyn DownloadVideoOutputBoundary>,

    downloader: Arc<dyn Downloader>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let url = model.url;
        
        // let playlist, playlist_snapshots, video_snapshots = 
    }
}
