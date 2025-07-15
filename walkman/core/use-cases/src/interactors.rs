use ::async_trait::async_trait;
use ::derive_new::new;
use ::std::sync::Arc;

use crate::boundaries::DownloadPlaylistInputBoundary;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoInputBoundary;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::gateways::Downloader;
use crate::gateways::MetadataWriter;
use crate::gateways::PlaylistDownloadEvent;
use crate::gateways::VideoDownloadEvent;
use crate::utils::aliases::Fallible;

#[derive(new)]
pub struct DownloadVideoInteractor {
    output_boundary: Arc<dyn DownloadVideoOutputBoundary>,

    downloader: Arc<dyn Downloader>,
    metadata_writer: Arc<dyn MetadataWriter>,
}

#[async_trait]
impl DownloadVideoInputBoundary for DownloadVideoInteractor {
    async fn apply(&self, model: DownloadVideoRequestModel) -> Fallible<()> {
        use ::futures_util::StreamExt as _;

        let DownloadVideoRequestModel {
            url,
            directory,
        } = model;

        let video_events = self.downloader.download_video(url, directory).await?;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        ::futures_util::pin_mut!(video_events);

        while let Some(event) = video_events.next().await {
            output_boundary.update(&event).await?;

            match event {
                VideoDownloadEvent::Completed(video) => {
                    metadata_writer.write_video(&video).await?;
                },
                _ => {},
            }
        }

        Ok(())
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
    async fn apply(&self, model: DownloadPlaylistRequestModel) -> Fallible<()> {
        use ::futures_util::StreamExt as _;

        let DownloadPlaylistRequestModel {
            url,
            directory,
        } = model;
        let (playlist_events, video_events) = self.downloader.download_playlist(url, directory).await?;

        let output_boundary = self.output_boundary.clone();
        let metadata_writer = self.metadata_writer.clone();

        let playlist_handle: ::tokio::task::JoinHandle<Fallible<()>> = ::tokio::spawn(async move {
            ::futures_util::pin_mut!(playlist_events);

            while let Some(event) = playlist_events.next().await {
                DownloadPlaylistOutputBoundary::update(&*output_boundary, &event).await?;

                match event {
                    PlaylistDownloadEvent::Completed(playlist) => {
                        metadata_writer.write_playlist(&playlist).await?;
                    },
                    _ => {},
                }
            }

            Ok(())
        });

        let output_boundary = self.output_boundary.clone();

        let video_handle: ::tokio::task::JoinHandle<Fallible<()>> = ::tokio::spawn(async move {
            ::futures_util::pin_mut!(video_events);

            while let Some(event) = video_events.next().await {
                DownloadVideoOutputBoundary::update(&*output_boundary, &event).await?;
            }

            Ok(())
        });

        let _ = ::tokio::try_join!(playlist_handle, video_handle)?;

        Ok(())
    }
}
