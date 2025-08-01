use ::async_trait::async_trait;

use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;

#[async_trait]
pub trait VideoDownloader: ::core::marker::Send + ::core::marker::Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, url: MaybeOwnedString,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PlaylistDownloader: ::core::marker::Send + ::core::marker::Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, url: MaybeOwnedString,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PostProcessor<Artifact>: ::core::marker::Send + ::core::marker::Sync {
    async fn process(self: ::std::sync::Arc<Self>, artifact: &Artifact) -> Fallible<()>;
}

#[async_trait]
pub trait UrlRepository: ::core::marker::Send + ::core::marker::Sync {
    async fn insert_video_url(self: ::std::sync::Arc<Self>, url: MaybeOwnedString) -> Fallible<()>;
    async fn insert_playlist_url(self: ::std::sync::Arc<Self>, url: MaybeOwnedString) -> Fallible<()>;

    async fn get_urls(self: ::std::sync::Arc<Self>) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>;
}
