pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

pub struct Video {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
    pub path: MaybeOwnedPath,
}

pub struct UnresolvedVideo {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
}

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

pub struct UnresolvedPlaylist {
    pub id: MaybeOwnedString,
    pub metadata: PlaylistMetadata,
}

pub struct PlaylistMetadata {
    pub title: MaybeOwnedString,
    pub size: usize,
}
