use ::async_trait::async_trait;

use crate::gateways::PlaylistDownloadEvent;
use crate::gateways::VideoDownloadEvent;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[async_trait]
pub trait DownloadVideoInputBoundary {
    async fn apply(&self, model: DownloadVideoRequestModel) -> Fallible<()>;
}

pub struct DownloadVideoRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

#[async_trait]
pub trait DownloadVideoOutputBoundary: Send + Sync {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()>;
}

#[async_trait]
pub trait DownloadPlaylistInputBoundary {
    async fn apply(&self, model: DownloadPlaylistRequestModel) -> Fallible<()>;
}

pub struct DownloadPlaylistRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

#[async_trait]
pub trait DownloadPlaylistOutputBoundary: DownloadVideoOutputBoundary {
    async fn update(&self, event: &PlaylistDownloadEvent) -> Fallible<()>;
}
