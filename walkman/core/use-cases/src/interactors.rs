use async_trait::async_trait;
use triomphe::Arc;

use crate::boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistProgressSnapshot, DownloadPlaylistRequestModel, DownloadVideoOutputBoundary};

pub struct DownloadPlaylistInteractor {
    download_playlist_output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,
    // download_video_output_boundary: SharedPointer<Box<dyn DownloadVideoOutputBoundary>, Kind>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let url = model.url;
        
        self.download_playlist_output_boundary.update(DownloadPlaylistProgressSnapshot::default()).await;

        
    }
}
