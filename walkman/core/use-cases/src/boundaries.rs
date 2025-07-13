use async_trait::async_trait;

use crate::{gateways::{PlaylistDownloadEvent, VideoDownloadEvent}, utils::aliases::{MaybeOwnedPath, MaybeOwnedString}};

#[async_trait]
pub trait DownloadVideoInputBoundary {
    async fn apply(&self, model: DownloadVideoRequestModel);
}

pub struct DownloadVideoRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

#[async_trait]
pub trait DownloadVideoOutputBoundary: Send + Sync {
    async fn update(&self, event: &VideoDownloadEvent);
}

#[async_trait]
pub trait DownloadPlaylistInputBoundary {
    async fn apply(&self, model: DownloadPlaylistRequestModel);
}

pub struct DownloadPlaylistRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

#[async_trait]
pub trait DownloadPlaylistOutputBoundary: DownloadVideoOutputBoundary {
    async fn update(&self, event: &PlaylistDownloadEvent);
}
