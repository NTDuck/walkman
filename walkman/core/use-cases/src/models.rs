use domain::{Playlist, UnresolvedPlaylist};
use ::domain::{UnresolvedVideo, Video};

use crate::utils::aliases::MaybeOwnedString;

#[derive(Debug)]
pub enum VideoEvent {
    Started(VideoStartedEvent),
    Downloading(VideoDownloadingEvent),
    Completed(VideoCompletedEvent),
    Warning(VideoWarningEvent),
    Failed(VideoFailedEvent),
}

#[derive(Debug)]
pub struct VideoStartedEvent {
    pub video: UnresolvedVideo,
}

#[derive(Debug)]
pub struct VideoDownloadingEvent {
    pub percentage: u8,

    pub eta: MaybeOwnedString,
    pub size: MaybeOwnedString,
    pub speed: MaybeOwnedString,
}

#[derive(Debug)]
pub struct VideoCompletedEvent {
    pub video: Video,
}

#[derive(Debug)]
pub struct VideoWarningEvent {
    pub message: MaybeOwnedString,
}

#[derive(Debug)]
pub struct VideoFailedEvent {
    pub message: MaybeOwnedString,
}


#[derive(Debug)]
pub enum PlaylistEvent {
    Started(PlaylistStartedEvent),
    VideoCompleted(PlaylistVideoDownloadedEvent),
    Completed(PlaylistCompletedEvent),
    Warning(PlaylistWarningEvent),
    Failed(PlaylistFailedEvent),
}

#[derive(Debug)]
pub struct PlaylistStartedEvent {
    pub playlist: UnresolvedPlaylist,
}

#[derive(Debug)]
pub struct PlaylistVideoDownloadedEvent {
    pub video: Video,
}

#[derive(Debug)]
pub struct PlaylistCompletedEvent {
    pub playlist: Playlist,
}

#[derive(Debug)]
pub struct PlaylistWarningEvent {
    pub message: MaybeOwnedString,
}

#[derive(Debug)]
pub struct PlaylistFailedEvent {
    pub message: MaybeOwnedString,
}
