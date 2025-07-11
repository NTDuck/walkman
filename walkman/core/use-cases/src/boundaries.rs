use async_trait::async_trait;

use crate::gateways::{PlaylistDownloadSnapshot, VideoDownloadSnapshot};

#[async_trait]
pub trait DownloadVideoInputBoundary {
    async fn apply(&self, model: DownloadVideoRequestModel);
}

pub struct DownloadVideoRequestModel {
    pub url: String,
}

#[async_trait]
pub trait DownloadVideoOutputBoundary: Send + Sync {
    async fn refresh(&self);
    async fn update(&self, snapshot: VideoDownloadSnapshot);
    async fn terminate(&self);
}

#[async_trait]
pub trait DownloadPlaylistInputBoundary {
    async fn apply(&self, model: DownloadPlaylistRequestModel);
}

pub struct DownloadPlaylistRequestModel {
    pub url: String,
}

#[async_trait]
pub trait DownloadPlaylistOutputBoundary: Send + Sync {
    async fn update(&self, snapshot: PlaylistDownloadSnapshot);
    async fn terminate(&self);
}
