use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use tokio::spawn;
use tracing::error;
use triomphe::Arc;

use crate::{boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistRequestModel, DownloadVideoOutputBoundary}, gateways::Downloader};

pub struct DownloadPlaylistInteractor {
    download_playlist_output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,
    download_video_output_boundary: Arc<dyn DownloadVideoOutputBoundary>,

    downloader: Arc<dyn Downloader>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let url = model.url;
        
        match self.downloader.download_playlist(url).await {
            Ok((_, playlist_snapshots, video_snapshots)) => {
                let download_playlist_output_boundary = self.download_playlist_output_boundary.clone();
                let download_video_output_boundary = self.download_video_output_boundary.clone();

                spawn(async move {
                    pin_mut!(playlist_snapshots);

                    while let Some(snapshot) = playlist_snapshots.next().await {
                        download_playlist_output_boundary.update(snapshot).await;
                    }

                    download_playlist_output_boundary.terminate().await;
                });

                spawn(async move {
                    pin_mut!(video_snapshots);

                    while let Some(snapshot) = video_snapshots.next().await {
                        download_video_output_boundary.update(snapshot).await;
                    }

                    download_video_output_boundary.terminate().await;
                });              
            },
            Err(error) => {
                error!("{:?}", error);
            },
        }
    }
}
