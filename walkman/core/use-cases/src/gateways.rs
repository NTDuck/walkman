use std::path::Path;

use async_trait::async_trait;
use futures_core::Stream;

use crate::{boundaries::{DownloadPlaylistProgressSnapshot, DownloadVideoProgressSnapshot}, utils::aliases::MaybeOwnedStr};

#[async_trait]
pub trait Downloader {
    async fn download_video(&self, url: MaybeOwnedStr) -> impl Stream<Item = DownloadVideoProgressSnapshot>;
    async fn download_playlist(&self, url: MaybeOwnedStr) -> impl Stream<Item = DownloadPlaylistProgressSnapshot>;
}

#[async_trait]
pub trait MetadataWriter {
    async fn set_album(&self, path: Path, album: MaybeOwnedStr);
    async fn set_artist(&self, path: Path, artist: MaybeOwnedStr);
    async fn set_genre(&self, path: Path, genre: MaybeOwnedStr);
}
