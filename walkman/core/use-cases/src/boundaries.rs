use ::async_trait::async_trait;

use crate::models::DownloadDiagnosticEvent;
use crate::models::PlaylistDownloadEvent;
use crate::models::VideoDownloadEvent;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

pub trait DownloadVideoInputBoundary: Accept<DownloadVideoRequestModel> {}

impl<InputBoundary> DownloadVideoInputBoundary for InputBoundary
where
    InputBoundary: Accept<DownloadVideoRequestModel>,
{
}

pub struct DownloadVideoRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

pub trait DownloadVideoOutputBoundary: Update<VideoDownloadEvent> + Update<DownloadDiagnosticEvent> {}

impl<OutputBoundary> DownloadVideoOutputBoundary for OutputBoundary
where
    OutputBoundary: Update<VideoDownloadEvent> + Update<DownloadDiagnosticEvent>,
{
}

pub trait DownloadPlaylistInputBoundary: Accept<DownloadPlaylistRequestModel> {}

impl<InputBoundary> DownloadPlaylistInputBoundary for InputBoundary
where
    InputBoundary: Accept<DownloadPlaylistRequestModel>,
{
}

pub struct DownloadPlaylistRequestModel {
    pub url: MaybeOwnedString,
    pub directory: MaybeOwnedPath,
}

pub trait DownloadPlaylistOutputBoundary: Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DownloadDiagnosticEvent> {}

impl<OutputBoundary> DownloadPlaylistOutputBoundary for OutputBoundary
where
    OutputBoundary: Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DownloadDiagnosticEvent>,
{
}

#[async_trait]
pub trait Accept<Request>: Send + Sync {
    async fn accept(&self, request: Request) -> Fallible<()>;
}

#[async_trait]
pub trait Update<Event>: Send + Sync {
    async fn update(&self, event: &Event) -> Fallible<()>;
}
