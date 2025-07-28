use ::async_trait::async_trait;

use crate::models::descriptors::UnresolvedPlaylist;
use crate::models::descriptors::UnresolvedVideo;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;

#[async_trait]
pub trait VideoDownloader<'a>: Send + Sync {
    async fn download(self: ::std::sync::Arc<Self>, video: UnresolvedVideo<'a>) -> Fallible<(BoxedStream<VideoDownloadEvent<'a>>, BoxedStream<DiagnosticEvent<'a>>)>;
}

#[async_trait]
pub trait PlaylistDownloader<'a>: Send + Sync {
    async fn download(self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist<'a>) -> Fallible<(BoxedStream<PlaylistDownloadEvent<'a>>, BoxedStream<VideoDownloadEvent<'a>>, BoxedStream<DiagnosticEvent<'a>>)>;
}
#[async_trait]
pub trait PostProcessor<Artifact>: Send + Sync {
    async fn process(self: ::std::sync::Arc<Self>, artifact: &Artifact) -> Fallible<()>;
}
