use async_trait::async_trait;

use crate::gateways::{PlaylistDownloadEvent, VideoDownloadEvent};

#[async_trait]
pub trait DownloadVideoInputBoundary {
    async fn apply(&self, model: DownloadVideoRequestModel);
}

pub struct DownloadVideoRequestModel {
    pub url: String,
}

#[async_trait]
pub trait DownloadVideoOutputBoundary: Send + Sync {
    async fn update(&self, event: VideoDownloadEvent);
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
    async fn update(&self, event: PlaylistDownloadEvent);
}
