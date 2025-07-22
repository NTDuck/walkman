use ::async_trait::async_trait;

use crate::models::descriptors::ResolvedPlaylist;
use crate::models::descriptors::ResolvedVideo;
use crate::models::events::DiagnosticEvent;
use crate::models::events::PlaylistDownloadEvent;
use crate::models::events::VideoDownloadEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(self: ::std::sync::Arc<Self>, url: MaybeOwnedString, directory: MaybeOwnedPath) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
    async fn download_playlist(self: ::std::sync::Arc<Self>, url: MaybeOwnedString, directory: MaybeOwnedPath) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)>;
}

#[async_trait]
pub trait MetadataWriter: Send + Sync {
    async fn write_video(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()>;

    async fn write_playlist(self: ::std::sync::Arc<Self>, playlist: &ResolvedPlaylist) -> Fallible<()> {
        use ::futures::StreamExt as _;

        let mut futures = playlist.videos
            .iter()
            .map(|video| ::std::sync::Arc::clone(&self).write_video(video))
            .collect::<::futures::stream::FuturesUnordered<_>>();

        // https://users.rust-lang.org/t/awaiting-futuresunordered/49295
        while (futures.next().await).is_some() {}

        Ok(())
    }
}
