use ::async_trait::async_trait;
use ::derive_new::new;

use crate::boundaries::Accept;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::boundaries::UpdateResourcesOutputBoundary;
use crate::boundaries::UpdateResourcesRequestModel;
use crate::gateways::PlaylistDownloader;
use crate::gateways::PostProcessor;
use crate::gateways::ResourceRepository;
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

    resources: ::std::sync::Arc<dyn ResourceRepository>,
    downloader: ::std::sync::Arc<dyn VideoDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadVideoRequestModel) -> Fallible<()> {
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        let DownloadVideoRequestModel { url } = request;
        let video = UnresolvedVideo { url };

        ::std::sync::Arc::clone(&self.resources).insert(video.clone()).await?;

        let (video_download_events, diagnostic_events) =
            ::std::sync::Arc::clone(&self.downloader).download(video).await?;

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

    resources: ::std::sync::Arc<dyn ResourceRepository>,
    downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        let DownloadPlaylistRequestModel { url } = request;
        let playlist = UnresolvedPlaylist { url };

        ::std::sync::Arc::clone(&self.resources).insert(playlist.clone()).await?;

        let (video_download_events, playlist_download_events, diagnostic_events) =
            ::std::sync::Arc::clone(&self.downloader).download(playlist).await?;

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
        use ::futures::StreamExt as _;

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

#[derive(new)]
pub struct UpdateResourcesInteractor {
    output_boundary: ::std::sync::Arc<dyn UpdateResourcesOutputBoundary>,

    resources: ::std::sync::Arc<dyn ResourceRepository>,

    video_downloader: ::std::sync::Arc<dyn VideoDownloader>,
    playlist_downloader: ::std::sync::Arc<dyn PlaylistDownloader>,

    video_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
    playlist_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<UpdateResourcesRequestModel> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: UpdateResourcesRequestModel) -> Fallible<()> {
        ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

        let (videos, playlists) = ::std::sync::Arc::clone(&self.resources).get_all().await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(videos),
            ::std::sync::Arc::clone(&self).accept(playlists),
        )?;

        ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<UnresolvedVideo>> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, videos: BoxedStream<UnresolvedVideo>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(videos);

        while let Some(video) = videos.next().await {
            ::std::sync::Arc::clone(&self).accept(video).await?
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<UnresolvedVideo> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, video: UnresolvedVideo) -> Fallible<()> {
        let (video_download_events, diagnostic_events) =
            ::std::sync::Arc::clone(&self.video_downloader).download(video).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<UnresolvedPlaylist>> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, playlists: BoxedStream<UnresolvedPlaylist>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(playlists);

        while let Some(playlist) = playlists.next().await {
            ::std::sync::Arc::clone(&self).accept(playlist).await?
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<UnresolvedPlaylist> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist) -> Fallible<()> {
        let (video_download_events, playlist_download_events, diagnostic_events) =
            ::std::sync::Arc::clone(&self.playlist_downloader).download(playlist).await?;

        ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).accept(video_download_events),
            ::std::sync::Arc::clone(&self).accept(playlist_download_events),
            ::std::sync::Arc::clone(&self).accept(diagnostic_events),
        )?;

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<VideoDownloadEvent>> for UpdateResourcesInteractor {
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
impl Accept<BoxedStream<PlaylistDownloadEvent>> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

            if let PlaylistDownloadEvent::Completed(event) = event {
                for postprocessor in self.playlist_postprocessors.iter() {
                    ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Accept<BoxedStream<DiagnosticEvent>> for UpdateResourcesInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, events: BoxedStream<DiagnosticEvent>) -> Fallible<()> {
        use ::futures::StreamExt as _;

        ::futures::pin_mut!(events);

        while let Some(event) = events.next().await {
            ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
        }

        Ok(())
    }
}
