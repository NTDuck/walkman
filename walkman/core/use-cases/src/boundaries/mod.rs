use async_trait::async_trait;

use crate::utils::MaybeOwnedStr;

#[async_trait]
pub trait DownloadPlaylistInputBoundary {
    async fn apply(&self, model: DownloadPlaylistRequestModel);
}

#[async_trait]
pub trait DownloadPlaylistOutputBoundary {
    async fn update(&self, model: DownloadPlaylistResponseModel);
}

pub struct DownloadPlaylistRequestModel {
    pub playlist_upstream_uri: MaybeOwnedStr,
}

pub struct DownloadPlaylistResponseModel {
    pub audio_title: MaybeOwnedStr,
    pub playlist_title: MaybeOwnedStr,
    pub download_speed: MaybeOwnedStr,
    pub audio_downloaded_bytes: u64,
    pub audio_total_bytes: u64,
    pub playlist_downloaded_audio: u64,
    pub playlist_total_audio: u64,
}

/*
Audio title (of current)
Playlist title
Download speed: B
Progress (of current): < 100, B/B
Progress (total): < 100, N/N
 */

/*
Audio title
Download speed: B
Progress: < 100, B/B
 */