use ::async_trait::async_trait;
use ::domain::Playlist;
use ::domain::Video;

use crate::models::PlaylistEvent;
use crate::models::VideoEvent;
use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<BoxedStream<VideoEvent>>;

    async fn download_playlist(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistEvent>, BoxedStream<VideoEvent>)>;
}

#[async_trait]
pub trait MetadataWriter: Send + Sync {
    async fn write_video(&self, video: &Video) -> Fallible<()>;

    async fn write_playlist(&self, playlist: &Playlist) -> Fallible<()> {
        use ::futures_util::StreamExt as _;

        let mut futures = playlist.videos
            .iter()
            .map(|video| self.write_video(video))
            .collect::<::futures_util::stream::FuturesUnordered<_>>();

        // https://users.rust-lang.org/t/awaiting-futuresunordered/49295
        while (futures.next().await).is_some() {}

        Ok(())
    }
}
