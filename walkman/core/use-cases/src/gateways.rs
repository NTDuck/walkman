use ::async_trait::async_trait;

use crate::models::descriptors::UnresolvedPlaylist;
use crate::models::descriptors::UnresolvedVideo;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;

#[async_trait]
pub trait VideoDownloader: Send + Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, video: UnresolvedVideo,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PlaylistDownloader: Send + Sync {
    async fn download(
        self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PostProcessor<Resource>: Send + Sync {
    async fn process(self: ::std::sync::Arc<Self>, resource: &Resource) -> Fallible<()>;
}

#[async_trait]
pub trait ResourceRepository: Insert<UnresolvedVideo> + Insert<UnresolvedPlaylist> {
    async fn get_all(self: ::std::sync::Arc<Self>) -> Fallible<(BoxedStream<UnresolvedVideo>, BoxedStream<UnresolvedPlaylist>)>;
}

#[async_trait]
pub trait Insert<Resource>: Send + Sync {
    async fn insert(self: ::std::sync::Arc<Self>, resource: Resource) -> Fallible<()>;
}
