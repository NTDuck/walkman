use ::async_trait::async_trait;

use crate::models::events::ChannelDownloadEvent;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;

pub trait DownloadVideoInputBoundary: Accept<DownloadVideoRequestModel> + ::core::marker::Send + ::core::marker::Sync {}

impl<InputBoundary> DownloadVideoInputBoundary for InputBoundary where
    InputBoundary: Accept<DownloadVideoRequestModel> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub struct DownloadVideoRequestModel {
    pub url: MaybeOwnedString,
}

pub trait DownloadPlaylistInputBoundary: Accept<DownloadPlaylistRequestModel> + ::core::marker::Send + ::core::marker::Sync {}

impl<InputBoundary> DownloadPlaylistInputBoundary for InputBoundary where
    InputBoundary: Accept<DownloadPlaylistRequestModel> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub struct DownloadPlaylistRequestModel {
    pub url: MaybeOwnedString,
}

pub trait DownloadChannelInputBoundary: Accept<DownloadChannelRequestModel> + ::core::marker::Send + ::core::marker::Sync {}

impl<InputBoundary> DownloadChannelInputBoundary for InputBoundary where
    InputBoundary: Accept<DownloadChannelRequestModel> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub struct DownloadChannelRequestModel {
    pub url: MaybeOwnedString,
}

pub trait UpdateMediaInputBoundary: Accept<UpdateMediaRequestModel> + ::core::marker::Send + ::core::marker::Sync {}

impl<InputBoundary> UpdateMediaInputBoundary for InputBoundary where
    InputBoundary: Accept<UpdateMediaRequestModel> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub struct UpdateMediaRequestModel;

pub trait DownloadVideoOutputBoundary: Activate + Update<VideoDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync {}

impl<OutputBoundary> DownloadVideoOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<VideoDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub trait DownloadPlaylistOutputBoundary:
    Activate + Update<VideoDownloadEvent> + Update<PlaylistDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync
{
}

impl<OutputBoundary> DownloadPlaylistOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<VideoDownloadEvent> + Update<PlaylistDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync,
{
}

pub trait UpdateMediaOutputBoundary:
    Activate + Update<VideoDownloadEvent> + Update<PlaylistDownloadEvent> + Update<ChannelDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync
{
}

impl<OutputBoundary> UpdateMediaOutputBoundary for OutputBoundary where
    OutputBoundary: Activate + Update<VideoDownloadEvent> + Update<PlaylistDownloadEvent> + Update<ChannelDownloadEvent> + Update<DiagnosticEvent> + ::core::marker::Send + ::core::marker::Sync,
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
