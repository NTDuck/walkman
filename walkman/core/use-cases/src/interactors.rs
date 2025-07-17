use ::async_trait::async_trait;
use ::derive_new::new;

use crate::boundaries::Accept;
use crate::boundaries::DownloadPlaylistOutputBoundary;
use crate::boundaries::DownloadPlaylistRequestModel;
use crate::boundaries::DownloadVideoOutputBoundary;
use crate::boundaries::DownloadVideoRequestModel;
use crate::gateways::Downloader;
use crate::gateways::MetadataWriter;
use crate::models::VideoDownloadEvent;
use crate::utils::aliases::Fallible;

#[derive(new)]
pub struct DownloadVideoInteractor {
    output_boundary: ::std::sync::Arc<dyn DownloadVideoOutputBoundary>,

    downloader: ::std::sync::Arc<dyn Downloader>,
    metadata_writer: ::std::sync::Arc<dyn MetadataWriter>,
}

#[async_trait]
impl Accept<DownloadVideoRequestModel> for DownloadVideoInteractor {
    async fn accept(&self, request: DownloadVideoRequestModel) -> Fallible<()> {
        let DownloadVideoRequestModel { url, directory } = request;
        let (video_event_stream, diagnostic_event_stream) = self.downloader.download_video(url, directory).await?;

        ::tokio::try_join!(
            self.accept(video_event_stream),
            self.accept(diagnostic_event_stream),
        )?;

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
    async fn accept(&self, request: DownloadPlaylistRequestModel) -> Fallible<()> {
        let DownloadPlaylistRequestModel { url, directory } = request;
        let (playlist_event_stream, video_event_streams, diagnostic_event_stream) = self.downloader.download_playlist(url, directory).await?;

        ::tokio::try_join!(
            self.accept(playlist_event_stream),
            self.accept(video_event_streams),
            self.accept(diagnostic_event_stream),
        )?;

        Ok(())
    }
}

mod private {
    use crate::{models::{DownloadDiagnosticEvent, PlaylistDownloadEvent}, utils::aliases::BoxedStream};

    use super::*;

    #[async_trait]
    impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadVideoInteractor {
        async fn accept(&self, event_stream: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
            use ::futures_util::StreamExt as _;
            
            ::futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                self.output_boundary.update(&event).await?;

                if let VideoDownloadEvent::Completed(event) = event {
                    self.metadata_writer.write_video(&event.video).await?;
                }
            }

            Ok(())
        }
    }

    #[async_trait]
    impl Accept<BoxedStream<DownloadDiagnosticEvent>> for DownloadVideoInteractor {
        async fn accept(&self, event_stream: BoxedStream<DownloadDiagnosticEvent>) -> Fallible<()> {
            use ::futures_util::StreamExt as _;

            ::futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                self.output_boundary.update(&event).await?;
            }

            Ok(())
        }
    }

    #[async_trait]
    impl Accept<BoxedStream<PlaylistDownloadEvent>> for DownloadPlaylistInteractor {
        async fn accept(&self, event_stream: BoxedStream<PlaylistDownloadEvent>) -> Fallible<()> {
            use ::futures_util::StreamExt as _;

            ::futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                self.output_boundary.update(&event).await?;

                if let PlaylistDownloadEvent::Completed(event) = event {
                    self.metadata_writer.write_playlist(&event.playlist).await?;
                }
            }

            Ok(())
        }
    }

    #[async_trait]
    impl Accept<::std::boxed::Box<[BoxedStream<VideoDownloadEvent>]>> for DownloadPlaylistInteractor {
        async fn accept(&self, event_streams: ::std::boxed::Box<[BoxedStream<VideoDownloadEvent>]>) -> Fallible<()> {
            let futures = event_streams
                .into_vec()
                .into_iter()
                .map(|stream| self.accept(stream))
                .collect::<Vec<_>>();

            ::futures_util::future::try_join_all(futures).await?;

            Ok(())
        }
    }

    #[async_trait]
    impl Accept<BoxedStream<VideoDownloadEvent>> for DownloadPlaylistInteractor {
        async fn accept(&self, event_stream: BoxedStream<VideoDownloadEvent>) -> Fallible<()> {
            use ::futures_util::StreamExt as _;

            ::futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                self.output_boundary.update(&event).await?;
            }

            Ok(())
        }
    }

    #[async_trait]
    impl Accept<BoxedStream<DownloadDiagnosticEvent>> for DownloadPlaylistInteractor {
        async fn accept(&self, event_stream: BoxedStream<DownloadDiagnosticEvent>) -> Fallible<()> {
            use ::futures_util::StreamExt as _;

            ::futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                self.output_boundary.update(&event).await?;
            }

            Ok(())
        }
    }
}
