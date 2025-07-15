use ::async_trait::async_trait;
use ::domain::Playlist;
use ::domain::Video;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(
        &self,
        url: MaybeOwnedString,
        directory: MaybeOwnedPath,
    ) -> Fallible<BoxedStream<VideoEvent>>;

    async fn download_playlist(
        &self,
        url: MaybeOwnedString,
        directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistEvent>, BoxedStream<VideoEvent>)>;
}

#[derive(Debug)]
pub enum VideoEvent {
    Downloading(VideoDownloadingEvent),
    Success(VideoSuccessEvent),
    Warning(VideoWarningEvent),
    Error(VideoErrorEvent),
}

#[derive(Debug)]
pub struct VideoDownloadingEvent {
    pub percentage: u8,

    pub eta: MaybeOwnedString,
    pub size: MaybeOwnedString,
    pub speed: MaybeOwnedString,
}

#[derive(Debug)]
pub struct VideoSuccessEvent {
    pub video: Video,
}

#[derive(Debug)]
pub struct VideoWarningEvent {
    pub message: MaybeOwnedString,
}

#[derive(Debug)]
pub struct VideoErrorEvent {
    pub message: MaybeOwnedString,
}

#[derive(Debug)]
pub enum PlaylistEvent {
    Downloading(PlaylistDownloadingEvent),
    Success(PlaylistSuccessEvent),
    Warning(PlaylistWarningEvent),
    Error(PlaylistErrorEvent),
}

#[derive(Debug)]
pub struct PlaylistDownloadingEvent {
    pub video: Video,

    pub downloaded: usize,
    pub total: usize,
}

#[derive(Debug)]
pub struct PlaylistSuccessEvent {
    pub playlist: Playlist,
}

#[derive(Debug)]
pub struct PlaylistWarningEvent {
    pub message: MaybeOwnedString,
}

#[derive(Debug)]
pub struct PlaylistErrorEvent {
    pub message: MaybeOwnedString,
}

#[async_trait]
pub trait MetadataWriter: Send + Sync {
    async fn write_video(&self, video: &Video) -> Fallible<()>;

    async fn write_playlist(&self, playlist: &Playlist) -> Fallible<()> {
        use ::futures_util::StreamExt as _;

        let mut futures = playlist
            .videos
            .iter()
            .map(|video| self.write_video(video))
            .collect::<::futures_util::stream::FuturesUnordered<_>>();

        // https://users.rust-lang.org/t/awaiting-futuresunordered/49295
        while (futures.next().await).is_some() {}

        Ok(())
    }
}
