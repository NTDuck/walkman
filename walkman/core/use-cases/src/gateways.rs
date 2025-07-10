use async_trait::async_trait;
use domain::{Playlist, Video};
use futures_core::Stream;

use crate::{boundaries::{DownloadPlaylistProgressSnapshot, DownloadVideoProgressSnapshot}};

#[async_trait]
pub trait Downloader {
    async fn download_video(&self, url: String) -> Result<(
            Video,
            impl Stream<Item = DownloadVideoProgressSnapshot>,
        ), DownloadError>;
    async fn download_playlist(&self, url: String) -> Result<(
            Playlist,
            impl Stream<Item = DownloadPlaylistProgressSnapshot>,
            impl Stream<Item = DownloadVideoProgressSnapshot>,
        ), DownloadError>;
}

pub struct VideoDownloadSnapshot {
    pub percentage: u8,
    pub eta: std::time::Duration,
    pub size: String,
    pub rate: String,
}

pub struct PlaylistDownloadSnapshot {
    pub downloaded: usize,
    pub total: usize,
}

pub enum DownloadError {
    
}

#[async_trait]
pub trait VideoMetadataWriter {
    async fn write(&self, video: Video);
}
