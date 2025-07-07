use crate::utils::aliases::MaybeOwnedStr;

mod utils;

pub struct Video {
    pub id: MaybeOwnedStr,
    pub title: MaybeOwnedStr,
    pub tags: Vec<MaybeOwnedStr>,

    pub channel_id: MaybeOwnedStr,
}

pub struct Playlist {

}

pub struct Channel {
    pub id: MaybeOwnedStr,
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