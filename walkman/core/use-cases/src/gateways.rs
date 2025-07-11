use std::pin::Pin;

use async_trait::async_trait;
use domain::{Playlist, Video};

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(&self, url: String) -> Result<(Video, Stream<VideoDownloadSnapshot>), DownloadError>;
    async fn download_playlist(&self, url: String) -> Result<(Playlist, Stream<PlaylistDownloadSnapshot>, Stream<VideoDownloadSnapshot>), DownloadError>;
}

pub type Stream<T> = Pin<Box<dyn futures_core::Stream<Item = T> + Send>>;

#[derive(Default)]
pub struct VideoDownloadSnapshot {
    pub percentage: u8,
    pub eta: std::time::Duration,
    pub size: String,
    pub rate: String,
}

// TODO consult docs for exact lim
#[derive(Default)]
pub struct PlaylistDownloadSnapshot {
    pub downloaded: usize,
    pub total: usize,
}

#[derive(Debug)]
pub enum DownloadError {

}

#[async_trait]
pub trait VideoMetadataWriter {
    async fn write(&self, video: Video);
}
