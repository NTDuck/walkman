use std::path::PathBuf;

#[derive(Default)]
pub struct Video {
    pub id: String,
    pub metadata: VideoMetadata,
    pub path: PathBuf,
}

#[derive(Default)]
pub struct VideoMetadata {
    pub title: String,

    pub album: String,
    pub artists: Vec<String>,
    pub genres: Vec<String>,
}

#[derive(Default)]
pub struct Playlist {
    pub id: String,
    pub metadata: PlaylistMetadata,
    pub videos: Vec<Video>,
}

#[derive(Default)]
pub struct PlaylistMetadata {
    pub title: String,
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