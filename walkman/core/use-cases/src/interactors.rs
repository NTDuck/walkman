use ::async_trait::async_trait;
use ::derive_new::new;

use crate::boundaries::Accept;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::gateways::Downloader;
use crate::gateways::MetadataWriter;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::PlaylistDownloadEventPayload;
use crate::models::events::VideoDownloadEvent;
use crate::models::events::VideoDownloadEventPayload;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;

#[derive(new)]
pub struct DownloadVideoInteractor {
    output_boundary: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    downloader: ::std::sync::Arc<dyn Downloader>,
    metadata_writer: ::std::sync::Arc<dyn MetadataWriter>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel) -> Fallible<()> {
        let DownloadVideoRequestModel { url, directory } = request;

        let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.downloader).download_video(url, directory).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
            
            if let VideoDownloadEventPayload::Completed(payload) = event.payload {
                ::std::sync::Arc::clone(&self.metadata_writer).write_video(&payload.video).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[derive(new)]
pub struct DownloadPlaylistInteractor {
    output_boundary: ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>,

    downloader: ::std::sync::Arc<dyn Downloader>,
    metadata_writer: ::std::sync::Arc<dyn MetadataWriter>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        let DownloadPlaylistRequestModel { url, directory } = request;
        let (playlist_download_events, video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.downloader).download_playlist(url, directory).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let PlaylistDownloadEventPayload::Completed(payload) = event.payload {
                ::std::sync::Arc::clone(&self.metadata_writer).write_playlist(&payload.playlist).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}
