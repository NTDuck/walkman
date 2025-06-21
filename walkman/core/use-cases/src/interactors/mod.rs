use async_trait::async_trait;

use crate::boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistRequestModel};

pub struct DownloadPlaylistInteractor {

}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {

    }
}
