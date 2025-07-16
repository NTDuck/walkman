use ::domain::UnresolvedVideo;
use ::domain::Video;
use ::domain::Playlist;
use ::domain::UnresolvedPlaylist;

use crate::utils::aliases::MaybeOwnedString;

pub enum VideoDownloadEvent {
    Started(VideoDownloadStartedEvent),
    ProgressUpdated(VideoDownloadProgressUpdatedEvent),
    Completed(VideoDownloadCompletedEvent),
}

pub struct VideoDownloadStartedEvent {
    pub video: UnresolvedVideo,
}

pub struct VideoDownloadProgressUpdatedEvent {
    pub percentage: u8,

    pub eta: MaybeOwnedString,
    pub size: MaybeOwnedString,
    pub speed: MaybeOwnedString,
}

pub struct VideoDownloadCompletedEvent {
    pub video: Video,
}

pub enum PlaylistDownloadEvent {
    Started(PlaylistDownloadStartedEvent),
    ProgressUpdated(PlaylistDownloadProgressUpdatedEvent),
    Completed(PlaylistDownloadCompletedEvent),
}

pub struct PlaylistDownloadStartedEvent {
    pub playlist: UnresolvedPlaylist,
}

pub struct PlaylistDownloadProgressUpdatedEvent {
    pub video: Video,

    pub completed: usize,
    pub total: usize,
}

pub struct PlaylistDownloadCompletedEvent {
    pub playlist: Playlist,
}

pub struct DownloadDiagnosticEvent {
    pub level: DiagnosticLevel,
    pub message: MaybeOwnedString,
}

pub enum DiagnosticLevel {
    Warning,
    Error,
}
