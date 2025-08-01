use ::async_trait::async_trait;
use ::domain::ChannelUrl;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;
use ::futures::prelude::*;

use crate::boundaries::Accept;
use crate::boundaries::DownloadChannelOutputBoundary;
use crate::boundaries::DownloadChannelRequestModel;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::boundaries::UpdateMediaOutputBoundary;
use crate::boundaries::UpdateMediaRequestModel;
use crate::gateways::ChannelDownloader;
use crate::gateways::PlaylistDownloader;
use crate::gateways::PostProcessor;
use crate::gateways::UrlRepository;
use crate::gateways::VideoDownloader;
use crate::models::descriptors::ResolvedChannel;
use crate::models::descriptors::ResolvedPlaylist;
use crate::models::descriptors::ResolvedVideo;
use crate::models::events::ChannelDownloadEvent;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedVec;

pub struct DownloadVideoInteractor {
    pub output_boundary: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    pub urls: ::std::sync::Arc<dyn UrlRepository>,
    pub downloader: ::std::sync::Arc<dyn VideoDownloader>,
    pub postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel) -> Fallible<()> {
        let url: VideoUrl = request.url.into();
        
        let (_, (video_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;
        
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
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let VideoDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.postprocessors {
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
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

pub struct DownloadPlaylistInteractor {
    pub output_boundary: ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>,

    pub urls: ::std::sync::Arc<dyn UrlRepository>,
    pub downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    pub postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        let url: PlaylistUrl = request.url.into();
        
        let (_, (video_download_events, playlist_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;
        
        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let PlaylistDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

pub struct DownloadChannelInteractor {
    pub output_boundary: ::std::sync::Arc<dyn DownloadChannelOutputBoundary>,

    pub urls: ::std::sync::Arc<dyn UrlRepository>,
    pub downloader: ::std::sync::Arc<dyn ChannelDownloader>,
    pub postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>>,
}

#[async_trait]
impl Accept<DownloadChannelRequestModel> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadChannelRequestModel) -> Fallible<()> {
        let url: ChannelUrl = request.url.into();
        
        let (_, (video_download_events, playlist_download_events, channel_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;
        
        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(channel_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<ChannelDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<ChannelDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let ChannelDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.channel).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

pub struct UpdateMediaInteractor {
    pub output_boundary: ::std::sync::Arc<dyn UpdateMediaOutputBoundary>,

    pub urls: ::std::sync::Arc<dyn UrlRepository>,

    pub video_downloader: ::std::sync::Arc<dyn VideoDownloader>,
    pub playlist_downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    pub channel_downloader: ::std::sync::Arc<dyn ChannelDownloader>,

    pub video_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
    pub playlist_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
    pub channel_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>>,
}

#[async_trait]
impl Accept<UpdateMediaRequestModel> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, _: UpdateMediaRequestModel) -> Fallible<()> {
        let (video_urls, playlist_urls, channel_urls) = ::std::sync::Arc::clone(&self.urls).get().await?;

        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_urls),
            ::std::sync::Arc::clone(&self).accept(playlist_urls),
            ::std::sync::Arc::clone(&self).accept(channel_urls),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoUrl>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, urls: BoxedStream<VideoUrl>) -> Fallible<()> {
        ::futures::pin_mut!(urls);

        while let Some(url) = urls.next().await {
            ::std::sync::Arc::clone(&self).accept(url).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<VideoUrl> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, url: VideoUrl) -> Fallible<()> {
        let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.video_downloader).download(url.clone()).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept((video_download_events, UsePreprocessors)),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistUrl>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, urls: BoxedStream<PlaylistUrl>) -> Fallible<()> {
        ::futures::pin_mut!(urls);

        while let Some(url) = urls.next().await {
            ::std::sync::Arc::clone(&self).accept(url).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<PlaylistUrl> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, url: PlaylistUrl) -> Fallible<()> {
        let (video_download_events, playlist_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.playlist_downloader).download(url.clone()).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept((playlist_download_events, UsePreprocessors)),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<ChannelUrl>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, urls: BoxedStream<ChannelUrl>) -> Fallible<()> {
        ::futures::pin_mut!(urls);

        while let Some(url) = urls.next().await {
            ::std::sync::Arc::clone(&self).accept(url).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<ChannelUrl> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, url: ChannelUrl) -> Fallible<()> {
        let (video_download_events, playlist_download_events, channel_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.channel_downloader).download(url.clone()).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept((channel_download_events, UsePreprocessors)),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<(BoxedStream<VideoDownloadEvent>, UsePreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<VideoDownloadEvent>, UsePreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let VideoDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.video_postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.video).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<(BoxedStream<PlaylistDownloadEvent>, UsePreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<PlaylistDownloadEvent>, UsePreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let PlaylistDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.playlist_postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<(BoxedStream<ChannelDownloadEvent>, UsePreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<ChannelDownloadEvent>, UsePreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let ChannelDownloadEvent::Completed(event) = event {
                for postprocessor in &*self.channel_postprocessors {
                    ::std::sync::Arc::clone(postprocessor).process(&event.channel).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}

struct UsePreprocessors;
