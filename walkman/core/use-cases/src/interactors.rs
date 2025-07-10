use async_trait::async_trait;
use triomphe::Arc;

use crate::boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistProgressSnapshot, DownloadPlaylistRequestModel, DownloadVideoOutputBoundary};

pub struct DownloadPlaylistInteractor {
    download_playlist_output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,
    donwload_video_output_boundary: Arc<dyn DownloadVideoOutputBoundary>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let url = model.url;
        
        
    }
}
