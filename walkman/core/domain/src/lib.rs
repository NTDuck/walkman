pub(crate) mod utils;

use crate::utils::aliases::{MaybeOwnedPath, MaybeOwnedString};

#[derive(Default)]
pub struct Video {
    pub id: MaybeOwnedString,
    pub metadata: VideoMetadata,
    pub path: MaybeOwnedPath,
}

#[derive(Default)]
pub struct VideoMetadata {
    pub title: MaybeOwnedString,

    pub album: MaybeOwnedString,
    pub artists: Vec<MaybeOwnedString>,
    pub genres: Vec<MaybeOwnedString>,
}

#[derive(Default)]
pub struct Playlist {
    pub id: MaybeOwnedString,
    pub metadata: PlaylistMetadata,
    pub videos: Vec<Video>,
}

#[derive(Default)]
pub struct PlaylistMetadata {
    pub title: MaybeOwnedString,
}


/*
Options:
--no-abort-on-error
--no-plugin-dirs
--flat-playlist
--color no_color
--min-filesize ???
--max-filesize 44.6M

Video only:
--no-playlist

Playlist only:
--yes-playlist

Update:
--download-archive [xxx] (will be a file in the current dir)
--no-break-on-existing


Initial check-log-stuff:
--dump-user-agent: 

Consider:
- skip livestreams.

*/