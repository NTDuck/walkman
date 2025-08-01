use ::async_trait::async_trait;

use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;

pub trait DownloadVideoInputBoundary: Accept<DownloadVideoRequestModel> {}

impl<InputBoundary> DownloadVideoInputBoundary for InputBoundary where
    InputBoundary: Accept<DownloadVideoRequestModel>,
{
}

pub struct DownloadVideoRequestModel {
    pub url: MaybeOwnedString,
}

pub trait DownloadPlaylistInputBoundary: Accept<DownloadPlaylistRequestModel> {}

impl<InputBoundary> DownloadPlaylistInputBoundary for InputBoundary where
    InputBoundary: Accept<DownloadPlaylistRequestModel>,
{
}

pub struct DownloadPlaylistRequestModel {
    pub url: MaybeOwnedString,
}

pub trait UpdateResourcesInputBoundary: Accept<UpdateResourcesRequestModel> {}

impl<InputBoundary> UpdateResourcesInputBoundary for InputBoundary where
    InputBoundary: Accept<UpdateResourcesRequestModel>,
{
}

pub struct UpdateResourcesRequestModel {}

pub trait DownloadVideoOutputBoundary: Activate + Update<VideoDownloadEvent> + Update<DiagnosticEvent> {}

impl<OutputBoundary> DownloadVideoOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<VideoDownloadEvent> + Update<DiagnosticEvent>,
{
}

pub trait DownloadPlaylistOutputBoundary:
    Activate + Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DiagnosticEvent>
{
}

impl<OutputBoundary> DownloadPlaylistOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DiagnosticEvent>,
{
}

pub trait UpdateResourcesOutputBoundary:
    Activate + Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DiagnosticEvent>
{
}

impl<OutputBoundary> UpdateResourcesOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<PlaylistDownloadEvent> + Update<VideoDownloadEvent> + Update<DiagnosticEvent>,
{
}

#[async_trait]
pub trait Activate: ::core::marker::Send + ::core::marker::Sync {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()>;
    async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()>;
}

#[async_trait]
pub trait Accept<Request>: ::core::marker::Send + ::core::marker::Sync {
    async fn accept(self: ::std::sync::Arc<Self>, request: Request) -> Fallible<()>;
}

#[async_trait]
pub trait Update<Event>: ::core::marker::Send + ::core::marker::Sync {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event) -> Fallible<()>;
}
