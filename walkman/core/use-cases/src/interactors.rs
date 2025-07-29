use ::async_trait::async_trait;
use ::derive_new::new;

use crate::boundaries::Accept;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::gateways::PlaylistDownloader;
use crate::gateways::PostProcessor;
use crate::gateways::VideoDownloader;
use crate::models::descriptors::ResolvedPlaylist;
use crate::models::descriptors::ResolvedVideo;
use crate::models::descriptors::UnresolvedPlaylist;
use crate::models::descriptors::UnresolvedVideo;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedVec;

#[derive(new)]
pub struct DownloadVideoInteractor {
    output_boundary: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    downloader: ::std::sync::Arc<dyn VideoDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor
{
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel) -> Fallible<()> {
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        let DownloadVideoRequestModel { url } = request;
        let video = UnresolvedVideo { url };

        let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.downloader).download(video).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

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
            
            if let VideoDownloadEvent::Completed(event) = event {
                for postprocessor in self.postprocessors.iter() {
                    ::std::sync::Arc::clone(postprocessor).process(&event.video).await?;
                }
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

    downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        let DownloadPlaylistRequestModel { url } = request;
        let playlist = UnresolvedPlaylist { url };

        let (playlist_download_events, video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.downloader).download(playlist).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

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

            if let PlaylistDownloadEvent::Completed(event) = event {
                for postprocessor in self.postprocessors.iter() {
                    ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
                }
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
