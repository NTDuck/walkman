pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct Video {
    pub url: MaybeOwnedString,

    pub id: VideoId,
    pub metadata: VideoMetadata,

    pub path: MaybeOwnedPath,
}

pub type VideoId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub title: MaybeOwnedString,
    pub album: MaybeOwnedString,
    pub artists: Vec<MaybeOwnedString>,
    pub genres: Vec<MaybeOwnedString>,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub url: MaybeOwnedString,

    pub id: PlaylistId,
    pub metadata: PlaylistMetadata,
    pub videos: Vec<Video>,
}

pub type PlaylistId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub title: MaybeOwnedString,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub url: MaybeOwnedString,

    pub id: ChannelId,
    pub metadata: ChannelMetadata,
    pub playlists: Vec<Playlist>,
    pub videos: Vec<Video>,
}

pub type ChannelId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    pub title: MaybeOwnedString,
}
