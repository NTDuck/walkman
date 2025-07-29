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
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait PostProcessor<Artifact>: Send + Sync {
    async fn process(self: ::std::sync::Arc<Self>, artifact: &Artifact) -> Fallible<()>;
}
