use std::{process::ExitCode, time::Duration};

use async_trait::async_trait;
use domain::{Playlist, Video};
use futures_util::{stream::FuturesUnordered, StreamExt};

use crate::utils::aliases::{BoxedStream, MaybeOwnedString};

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(&self, url: MaybeOwnedString) -> BoxedStream<VideoDownloadEvent>;
    async fn download_playlist(&self, url: MaybeOwnedString) -> (BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>);
}

pub enum VideoDownloadEvent {
    Downloading {
        percentage: u8,
        eta: Duration,
        size: MaybeOwnedString,
        rate: MaybeOwnedString,
    },
    Completed(Video),
    Failed(ExitCode),
}

pub enum PlaylistDownloadEvent {
    Downloading {
        video: Video,

        downloaded: usize,
        total: usize,
    },
    Completed(Playlist),
    Failed(ExitCode),
}

#[async_trait]
pub trait MetadataWriter: Send + Sync {
    async fn write_video(&self, video: &Video);

    async fn write_playlist(&self, playlist: &Playlist) {
        let mut futures = playlist.videos
            .iter()
            .map(|video| self.write_video(video))
            .collect::<FuturesUnordered<_>>();

        // https://users.rust-lang.org/t/awaiting-futuresunordered/49295
        while let Some(_) = futures.next().await {}
    }
}
