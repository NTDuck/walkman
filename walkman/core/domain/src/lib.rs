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
    pub title: Option<MaybeOwnedString>,
    pub album: Option<MaybeOwnedString>,
    pub artists: Option<Vec<MaybeOwnedString>>,
    pub genres: Option<Vec<MaybeOwnedString>>,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub url: MaybeOwnedString,

    pub id: PlaylistId,
    pub metadata: PlaylistMetadata,
    pub videos: Option<Vec<Video>>,
}

pub type PlaylistId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub title: Option<MaybeOwnedString>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub url: MaybeOwnedString,

    pub id: ChannelId,
    pub metadata: ChannelMetadata,
    pub playlists: Option<Vec<Playlist>>,
    pub videos: Option<Vec<Video>>,
}

pub type ChannelId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    pub title: Option<MaybeOwnedString>,
}
