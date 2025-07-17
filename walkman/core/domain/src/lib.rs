pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[derive(Clone)]
pub struct Video {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
    pub path: MaybeOwnedPath,
}

pub struct UnresolvedVideo {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
}

#[derive(Clone)]
pub struct VideoMetadata {
    pub title: MaybeOwnedString,

    pub album: MaybeOwnedString,
    pub artists: Vec<MaybeOwnedString>,
    pub genres: Vec<MaybeOwnedString>,
}

pub struct Playlist {
    pub id: MaybeOwnedString,
    pub metadata: PlaylistMetadata,
    pub videos: Vec<Video>,
}

#[derive(Clone)]
pub struct UnresolvedPlaylist {
    pub id: MaybeOwnedString,
    pub metadata: PlaylistMetadata,
}

#[derive(Clone)]
pub struct PlaylistMetadata {
    pub title: MaybeOwnedString,
    pub video_urls: Vec<MaybeOwnedString>,
}
