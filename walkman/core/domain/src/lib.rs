pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[derive(Debug)]
pub struct Video {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
    pub path: MaybeOwnedPath,
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub title: MaybeOwnedString,

    pub album: MaybeOwnedString,
    pub artists: Vec<MaybeOwnedString>,
    pub genres: Vec<MaybeOwnedString>,
}

#[derive(Debug)]
pub struct Playlist {
    pub id: MaybeOwnedString,
    pub metadata: PlaylistMetadata,
    pub videos: Vec<Video>,
}

#[derive(Debug)]
pub struct PlaylistMetadata {
    pub title: MaybeOwnedString,
}
