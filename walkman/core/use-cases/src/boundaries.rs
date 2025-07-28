use ::async_trait::async_trait;

use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;

pub trait DownloadVideoInputBoundary: for<'a> Accept<DownloadVideoRequestModel<'a>> {}

impl<InputBoundary> DownloadVideoInputBoundary for InputBoundary
where
    InputBoundary: for<'a> Accept<DownloadVideoRequestModel<'a>>,
{
}

pub struct DownloadVideoRequestModel<'a> {
    pub url: MaybeOwnedString<'a>,
}

pub trait DownloadPlaylistInputBoundary: for<'a> Accept<DownloadPlaylistRequestModel<'a>> {}

impl<InputBoundary> DownloadPlaylistInputBoundary for InputBoundary
where
    InputBoundary: for<'a> Accept<DownloadPlaylistRequestModel<'a>>,
{
}

pub struct DownloadPlaylistRequestModel<'a> {
    pub url: MaybeOwnedString<'a>,
}

pub trait DownloadVideoOutputBoundary: Activate + for<'a> Update<VideoDownloadEvent<'a>> + for<'a> Update<DiagnosticEvent<'a>> {}

impl<OutputBoundary> DownloadVideoOutputBoundary for OutputBoundary
where
    OutputBoundary: Activate + for<'a> Update<VideoDownloadEvent<'a>> + for<'a> Update<DiagnosticEvent<'a>>,
{
}

pub trait DownloadPlaylistOutputBoundary: Activate + for<'a> Update<PlaylistDownloadEvent<'a>> + for<'a> Update<VideoDownloadEvent<'a>> + for<'a> Update<DiagnosticEvent<'a>> {}

impl<OutputBoundary> DownloadPlaylistOutputBoundary for OutputBoundary
where
    OutputBoundary: Activate + for<'a> Update<PlaylistDownloadEvent<'a>> + for<'a> Update<VideoDownloadEvent<'a>> + for<'a> Update<DiagnosticEvent<'a>>,
{
}

#[async_trait]
pub trait Activate: Send + Sync {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()>;
    async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()>;
}

#[async_trait]
pub trait Accept<Request>: Send + Sync {
    async fn accept(self: ::std::sync::Arc<Self>, request: Request) -> Fallible<()>;
}

#[async_trait]
pub trait Update<Event>: Send + Sync {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event) -> Fallible<()>;
}
