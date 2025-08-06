use ::async_trait::async_trait;
use ::domain::ChannelUrl;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;

use crate::models::events::ChannelDownloadEvent;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;

#[async_trait]
pub trait VideoDownloader: ::core::marker::Send + ::core::marker::Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, url: VideoUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PlaylistDownloader: ::core::marker::Send + ::core::marker::Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, url: PlaylistUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait ChannelDownloader: ::core::marker::Send + ::core::marker::Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, url: ChannelUrl,
    ) -> Fallible<(
        BoxedStream<VideoDownloadEvent>,
        BoxedStream<PlaylistDownloadEvent>,
        BoxedStream<ChannelDownloadEvent>,
        BoxedStream<DiagnosticEvent>,
    )>;
}

#[async_trait]
pub trait PostProcessor<Artifact>: ::core::marker::Send + ::core::marker::Sync {
    async fn process(self: ::std::sync::Arc<Self>, artifact: &Artifact) -> Fallible<()>;
}

#[async_trait]
pub trait UrlRepository:
    Insert<VideoUrl> + Insert<PlaylistUrl> + Insert<ChannelUrl> + ::core::marker::Send + ::core::marker::Sync
{
    async fn values(
        self: ::std::sync::Arc<Self>,
    ) -> Fallible<(BoxedStream<VideoUrl>, BoxedStream<PlaylistUrl>, BoxedStream<ChannelUrl>)>;
}

#[async_trait]
pub trait Insert<Item>: ::core::marker::Send + ::core::marker::Sync {
    async fn insert(self: ::std::sync::Arc<Self>, item: Item) -> Fallible<()>;
}
