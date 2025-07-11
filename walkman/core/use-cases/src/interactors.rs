use async_trait::async_trait;
use futures_util::{pin_mut, StreamExt};
use tokio::spawn;
use triomphe::Arc;

use crate::{boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistRequestModel, DownloadVideoInputBoundary, DownloadVideoOutputBoundary, DownloadVideoRequestModel}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

pub struct DownloadVideoInteractor {
    output_boundary: Arc<dyn DownloadVideoOutputBoundary>,

    downloader: Arc<dyn Downloader>,
    metadata_writer: Arc<dyn MetadataWriter>,   
}

#[async_trait]
impl DownloadVideoInputBoundary for DownloadVideoInteractor {
    async fn apply(&self, model: DownloadVideoRequestModel) {
        let url = model.url;

        let video_events = self.downloader.download_video(url).await;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        spawn(async move {
            pin_mut!(video_events);

            while let Some(event) = video_events.next().await {
                output_boundary.on_video_event(&event).await;

                match event {
                    VideoDownloadEvent::Completed(video) => {
                        metadata_writer.write_video(&video).await;
                    },
                    _ => {},
                }
            }
        });
    }
}

pub struct DownloadPlaylistInteractor {
    output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,

    downloader: Arc<dyn Downloader>,
    metadata_writer: Arc<dyn MetadataWriter>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let url = model.url;

        let (playlist_events, video_events) = self.downloader.download_playlist(url).await;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        spawn(async move {
            pin_mut!(playlist_events);

            while let Some(event) = playlist_events.next().await {
                output_boundary.on_playlist_event(&event).await;

                match event {
                    PlaylistDownloadEvent::Completed(playlist) => {
                        metadata_writer.write_playlist(&playlist).await;
                    },
                    _ => {},
                }
            }
        });

        let output_boundary = self.output_boundary.clone();

        spawn(async move {
            pin_mut!(video_events);

            while let Some(event) = video_events.next().await {
                output_boundary.on_video_event(&event).await;
            }
        });
    }
}
