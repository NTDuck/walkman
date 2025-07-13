use std::sync::Arc;

use async_trait::async_trait;
use derive_new::new;
use futures_util::{pin_mut, StreamExt};
use tokio::{join, spawn};

use crate::{boundaries::{DownloadPlaylistInputBoundary, DownloadPlaylistOutputBoundary, DownloadPlaylistRequestModel, DownloadVideoInputBoundary, DownloadVideoOutputBoundary, DownloadVideoRequestModel}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

#[derive(new)]
pub struct DownloadVideoInteractor {
    output_boundary: Arc<dyn DownloadVideoOutputBoundary>,

    downloader: Arc<dyn Downloader>,
    metadata_writer: Arc<dyn MetadataWriter>,   
}

#[async_trait]
impl DownloadVideoInputBoundary for DownloadVideoInteractor {
    async fn apply(&self, model: DownloadVideoRequestModel) {
        let DownloadVideoRequestModel { url, directory } = model;
        let video_events = self.downloader.download_video(url, directory).await;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        spawn(async move {
            pin_mut!(video_events);

            while let Some(event) = video_events.next().await {
                output_boundary.update(&event).await;

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

#[derive(new)]
pub struct DownloadPlaylistInteractor {
    output_boundary: Arc<dyn DownloadPlaylistOutputBoundary>,

    downloader: Arc<dyn Downloader>,
    metadata_writer: Arc<dyn MetadataWriter>,
}

#[async_trait]
impl DownloadPlaylistInputBoundary for DownloadPlaylistInteractor {
    async fn apply(&self, model: DownloadPlaylistRequestModel) {
        let DownloadPlaylistRequestModel { url, directory } = model;
        let (playlist_events, video_events) = self.downloader.download_playlist(url, directory).await;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        let playlist_handle = spawn(async move {
            pin_mut!(playlist_events);

            while let Some(event) = playlist_events.next().await {
                DownloadPlaylistOutputBoundary::update(&*output_boundary, &event).await;

                match event {
                    PlaylistDownloadEvent::Completed(playlist) => {
                        metadata_writer.write_playlist(&playlist).await;
                    },
                    _ => {},
                }
            }
        });

        let output_boundary = self.output_boundary.clone();

        let video_handle = spawn(async move {
            pin_mut!(video_events);

            while let Some(event) = video_events.next().await {
                DownloadVideoOutputBoundary::update(&*output_boundary, &event).await;
            }
        });

        let _ = join!(playlist_handle, video_handle);
    }
}
