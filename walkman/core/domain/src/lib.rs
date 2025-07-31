pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;

#[derive(Debug, Clone)]
pub struct Video {
    pub id: VideoId,
    pub url: MaybeOwnedString,

    pub metadata: VideoMetadata,

    pub path: MaybeOwnedPath,
}

pub type VideoId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub title: Option<MaybeOwnedString>,
    pub album: Option<MaybeOwnedString>,
    pub artists: Option<MaybeOwnedVec<MaybeOwnedString>>,
    pub genres: Option<MaybeOwnedVec<MaybeOwnedString>>,
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: PlaylistId,
    pub url: MaybeOwnedString,

    pub metadata: PlaylistMetadata,

    pub videos: Option<MaybeOwnedVec<Video>>,
}

pub type PlaylistId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub title: Option<MaybeOwnedString>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: ChannelId,
    pub url: MaybeOwnedString,

    pub metadata: ChannelMetadata,
    
    pub videos: Option<MaybeOwnedVec<Video>>,
    pub playlists: Option<MaybeOwnedVec<Playlist>>,
}

pub type ChannelId = MaybeOwnedString;

#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    pub title: Option<MaybeOwnedString>,
}
