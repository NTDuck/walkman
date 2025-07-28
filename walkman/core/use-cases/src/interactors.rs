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

#[derive(new)]
pub struct DownloadVideoInteractor<'a> {
    output_boundary: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    downloader: ::std::sync::Arc<dyn VideoDownloader<'a>>,
    postprocessors: Vec<::std::sync::Arc<dyn for<'b> PostProcessor<ResolvedVideo<'b>>>>,
}

#[async_trait]
impl<'a> Accept<DownloadVideoRequestModel<'a>> for DownloadVideoInteractor<'a>
{
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel<'a>) -> Fallible<()> {
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
impl<'a> Accept<BoxedStream<VideoDownloadEvent<'a>>> for DownloadVideoInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent<'a>>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
            
            if let VideoDownloadEvent::Completed(event) = event {
                for postprocessor in &self.postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.video).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Accept<BoxedStream<DiagnosticEvent<'a>>> for DownloadVideoInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent<'a>>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[derive(new)]
pub struct DownloadPlaylistInteractor<'a> {
    output_boundary: ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>,

    downloader: ::std::sync::Arc<dyn PlaylistDownloader<'a>>,
    postprocessors: Vec<::std::sync::Arc<dyn for<'b> PostProcessor<ResolvedPlaylist<'b>>>>,
}

#[async_trait]
impl<'a> Accept<DownloadPlaylistRequestModel<'a>> for DownloadPlaylistInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel<'a>) -> Fallible<()> {
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
impl<'a> Accept<BoxedStream<PlaylistDownloadEvent<'a>>> for DownloadPlaylistInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent<'a>>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let PlaylistDownloadEvent::Completed(event) = event {
                for postprocessor in &self.postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Accept<BoxedStream<VideoDownloadEvent<'a>>> for DownloadPlaylistInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent<'a>>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Accept<BoxedStream<DiagnosticEvent<'a>>> for DownloadPlaylistInteractor<'a> {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent<'a>>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}
