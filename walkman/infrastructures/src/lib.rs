mod utils;

use async_stream::stream;
use async_trait::async_trait;
use use_cases::gateways::{Downloader, PlaylistDownloadEvent, VideoDownloadEvent};

use crate::utils::aliases::{BoxedStream, MaybeOwnedString};

pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(&self, _url: MaybeOwnedString) -> BoxedStream<VideoDownloadEvent> {
        Box::pin(stream! {
            yield VideoDownloadEvent::Failed(Default::default());
        })
    }

    async fn download_playlist(&self, _url: MaybeOwnedString) -> (BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>) {
        (
            Box::pin(stream! {
                yield PlaylistDownloadEvent::Failed(Default::default());
            }),
            Box::pin(stream! {
                yield VideoDownloadEvent::Failed(Default::default());
            }),
        )
    }
}
