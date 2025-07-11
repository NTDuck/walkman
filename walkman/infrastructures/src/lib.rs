mod utils;

use async_stream::stream;
use async_trait::async_trait;
use domain::Video;
use use_cases::{boundaries::{DownloadPlaylistOutputBoundary, DownloadVideoOutputBoundary}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

use crate::utils::aliases::{BoxedStream, MaybeOwnedString};

pub struct DownloadVideoProgressBar;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadVideoProgressBar {
    async fn update(&self, _event: &VideoDownloadEvent) {

    }
}

pub struct DownloadPlaylistAndVideoProgressBar;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadPlaylistAndVideoProgressBar {
    async fn update(&self, _event: &VideoDownloadEvent) {
        
    }
}

#[async_trait]
impl DownloadPlaylistOutputBoundary for DownloadPlaylistAndVideoProgressBar {
    async fn update(&self, _event: &PlaylistDownloadEvent) {

    }
}

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

pub struct LoftyMetadataWriter;

#[async_trait]
impl MetadataWriter for LoftyMetadataWriter {
    async fn write_video(&self, video: &Video) {
        
    }
}
