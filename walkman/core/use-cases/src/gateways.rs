use async_trait::async_trait;
use futures_core::Stream;

use crate::{boundaries::{DownloadPlaylistProgressSnapshot, DownloadVideoProgressSnapshot}, utils::MaybeOwnedStr};

#[async_trait]
pub trait Downloader {
    async fn download_video(&self, url: MaybeOwnedStr) -> impl Stream<Item = DownloadVideoProgressSnapshot>;
    async fn download_playlist(&self, url: MaybeOwnedStr) -> impl Stream<Item = DownloadPlaylistProgressSnapshot>;
}
