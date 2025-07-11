use std::{pin::Pin, process::ExitCode};

use async_trait::async_trait;
use domain::{Playlist, Video};
use futures_util::{stream::FuturesUnordered, StreamExt};

#[async_trait]
pub trait Downloader: Send + Sync {
    async fn download_video(&self, url: String) -> Stream<VideoDownloadEvent>;

    async fn download_playlist(&self, url: String) -> (Stream<PlaylistDownloadEvent>, Stream<VideoDownloadEvent>);
}

pub type Stream<T> = Pin<Box<dyn futures_core::Stream<Item = T> + Send>>;

pub enum VideoDownloadEvent {
    Downloading {
        percentage: u8,
        eta: std::time::Duration,
        size: String,
        rate: String,
    },
    Completed(Video),
    Failed(ExitCode),
}

pub enum PlaylistDownloadEvent {
    Downloading {
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
        while let Some(()) = futures.next().await {}
    }
}
