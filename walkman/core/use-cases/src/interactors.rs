use ::async_trait::async_trait;
use domain::PlaylistUrl;
use domain::VideoUrl;
use ::futures::prelude::*;

use crate::boundaries::Accept;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::boundaries::UpdateMediaInputBoundary;
use crate::boundaries::UpdateMediaRequestModel;
use crate::gateways::PlaylistDownloader;
use crate::gateways::PostProcessor;
use crate::gateways::UrlRepository;
use crate::gateways::VideoDownloader;
use crate::models::descriptors::ResolvedPlaylist;
use crate::models::descriptors::ResolvedVideo;
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

    pub resources: ::std::sync::Arc<dyn UrlRepository>,
    pub downloader: ::std::sync::Arc<dyn PlaylistDownloader>,
    pub postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
}

#[async_trait]
impl Accept<DownloadPlaylistRequestModel> for DownloadPlaylistInteractor {
    async fn accept(self: ::std::sync::Arc<Self>, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        let url: PlaylistUrl = request.url.into();
        
        let (_, (video_download_events, playlist_download_events, diagnostic_events)) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self.resources).insert(url.clone()),
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

// pub struct UpdateResourcesInteractor {
//     pub output_boundary: ::std::sync::Arc<dyn UpdateMediaInputBoundary>,

//     pub urls: ::std::sync::Arc<dyn UrlRepository>,

//     pub video_downloader: ::std::sync::Arc<dyn VideoDownloader>,
//     pub playlist_downloader: ::std::sync::Arc<dyn PlaylistDownloader>,

//     pub video_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>>,
//     pub playlist_postprocessors: MaybeOwnedVec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>>,
// }

// #[async_trait]
// impl Accept<UpdateMediaRequestModel> for UpdateResourcesInteractor {
//     async fn accept(self: ::std::sync::Arc<Self>, _: UpdateMediaRequestModel) -> Fallible<()> {
//         ::std::sync::Arc::clone(&self.output_boundary).activate().await?;

//         let (video_urls, playlist_urls) = ::std::sync::Arc::clone(&self.urls).get().await?;

//         ::tokio::try_join!(
//             async {
//                 ::futures::pin_mut!(video_urls);

//                 while let Some(url) = video_urls.next().await {
//                     let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.video_downloader).download(url.clone()).await?;

//                     ::tokio::try_join!(
//                         async {
//                             ::futures::pin_mut!(video_download_events);

//                             while let Some(event) = video_download_events.next().await {
//                                 ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

//                                 if let VideoDownloadEvent::Completed(event) = event {
//                                     for postprocessor in self.video_postprocessors.iter() {
//                                         ::std::sync::Arc::clone(postprocessor).process(&event.video).await?;
//                                     }
//                                 }
//                             }

//                             Ok::<_, ::anyhow::Error>(())
//                         },

//                         async {
//                             ::futures::pin_mut!(diagnostic_events);

//                             while let Some(event) = diagnostic_events.next().await {
//                                 ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
//                             }

//                             Ok::<_, ::anyhow::Error>(())
//                         }
//                     )?;
//                 }

//                 Ok::<_, ::anyhow::Error>(())
//             },

//             async {
//                 ::futures::pin_mut!(playlist_urls);

//                 while let Some(url) = playlist_urls.next().await {
//                     let (video_download_events, playlist_download_events, diagnostic_events) = ::std::sync::Arc::clone(&self.playlist_downloader).download(url.clone()).await?;

//                     ::tokio::try_join!(
//                         async {
//                             ::futures::pin_mut!(video_download_events);

//                             while let Some(event) = video_download_events.next().await {
//                                 ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
//                             }

//                             Ok::<_, ::anyhow::Error>(())
//                         },

//                         async {
//                             ::futures::pin_mut!(playlist_download_events);

//                             while let Some(event) = playlist_download_events.next().await {
//                                 ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;

//                                 if let PlaylistDownloadEvent::Completed(event) = event {
//                                     for postprocessor in self.playlist_postprocessors.iter() {
//                                         ::std::sync::Arc::clone(postprocessor).process(&event.playlist).await?;
//                                     }
//                                 }
//                             }

//                             Ok::<_, ::anyhow::Error>(())
//                         },

//                         async {
//                             ::futures::pin_mut!(diagnostic_events);

//                             while let Some(event) = diagnostic_events.next().await {
//                                 ::std::sync::Arc::clone(&self.output_boundary).update(&event).await?;
//                             }

//                             Ok::<_, ::anyhow::Error>(())
//                         },
//                     )?;
//                 }

//                 Ok::<_, ::anyhow::Error>(())
//             },
//         )?;

//         ::std::sync::Arc::clone(&self.output_boundary).deactivate().await?;

//         Ok(())
//     }
// }
