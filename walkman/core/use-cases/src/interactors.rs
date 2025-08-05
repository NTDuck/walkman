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

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct DownloadVideoInteractor {
    view: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    urls: ::std::sync::Arc<dyn UrlRepository>,
    downloader: ::std::sync::Arc<dyn VideoDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel) -> Fallible<()> {
        let url: VideoUrl = request.url.into();
        
        let (_, (video_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.view).activate().await?;
        
        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.view).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;

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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct DownloadPlaylistInteractor {
    view: ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>,

    urls: ::std::sync::Arc<dyn UrlRepository>,
    downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        let url: PlaylistUrl = request.url.into();
        
        let (_, (video_download_events, playlist_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.view).activate().await?;
        
        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.view).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;

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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct DownloadChannelInteractor {
    view: ::std::sync::Arc<dyn DownloadChannelOutputBoundary>,

    urls: ::std::sync::Arc<dyn UrlRepository>,
    downloader: ::std::sync::Arc<dyn ChannelDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>>,
}

#[async_trait]
impl Accept<DownloadChannelRequestModel> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadChannelRequestModel) -> Fallible<()> {
        let url: ChannelUrl = request.url.into();
        
        let (_, (video_download_events, playlist_download_events, channel_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.urls).insert(url.clone()),
            ::std::sync::Arc::clone(&self.downloader).download(url.clone()),
        )?;
        
        ::std::sync::Arc::clone(&self.view).activate().await?;
        
        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(channel_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        ::std::sync::Arc::clone(&self.view).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<ChannelDownloadEvent>> for DownloadChannelInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<ChannelDownloadEvent>) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;

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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct UpdateMediaInteractor {
    view: ::std::sync::Arc<dyn UpdateMediaOutputBoundary>,

    urls: ::std::sync::Arc<dyn UrlRepository>,

    video_downloader: ::std::sync::Arc<dyn VideoDownloader>,
    playlist_downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    channel_downloader: ::std::sync::Arc<dyn ChannelDownloader>,

    video_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
    playlist_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
    channel_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>>,
}

#[async_trait]
impl Accept<UpdateMediaRequestModel> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, _: UpdateMediaRequestModel) -> Fallible<()> {
        let (video_urls, playlist_urls, channel_urls) = ::std::sync::Arc::clone(&self.urls).values().await?;

        ::std::sync::Arc::clone(&self.view).activate().await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_urls),
            ::std::sync::Arc::clone(&self).accept(playlist_urls),
            ::std::sync::Arc::clone(&self).accept(channel_urls),
        )?;

        ::std::sync::Arc::clone(&self.view).deactivate().await?;

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
            ::std::sync::Arc::clone(&self).accept((video_download_events, WithPreprocessors)),
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
            ::std::sync::Arc::clone(&self).accept((playlist_download_events, WithPreprocessors)),
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
            ::std::sync::Arc::clone(&self).accept((channel_download_events, WithPreprocessors)),
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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<(BoxedStream<VideoDownloadEvent>, WithPreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<VideoDownloadEvent>, WithPreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.view).update(&event).await?;

            if let VideoDownloadEvent::Completed(event) = event {
                ::tracing::debug!("Received (IB) event `{:?}`", event);  

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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<(BoxedStream<PlaylistDownloadEvent>, WithPreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<PlaylistDownloadEvent>, WithPreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;

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
impl Accept<(BoxedStream<ChannelDownloadEvent>, WithPreprocessors)> for UpdateMediaInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, (events, _): (BoxedStream<ChannelDownloadEvent>, WithPreprocessors)) -> Fallible<()> {
        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;

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
            ::tracing::debug!("Received (IB) event `{:?}`", event); 

            ::std::sync::Arc::clone(&self.view).update(&event).await?;
        }

        Ok(())
    }
}

struct WithPreprocessors;
