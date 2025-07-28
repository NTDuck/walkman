pub mod events {
    use ::domain::VideoId;

    use crate::{models::descriptors::{PartiallyResolvedPlaylist, PartiallyResolvedVideo, ResolvedPlaylist, ResolvedVideo}, utils::aliases::MaybeOwnedString};

    #[derive(Debug, Clone)]
    pub enum VideoDownloadEvent {
        Started(VideoDownloadStartedEvent),
        ProgressUpdated(VideoDownloadProgressUpdatedEvent),
        Completed(VideoDownloadCompletedEvent),
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadStartedEvent {
        pub video: PartiallyResolvedVideo,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadProgressUpdatedEvent {
        pub id: VideoId,

        pub eta: ::std::time::Duration,
        pub elapsed: ::std::time::Duration,

        pub downloaded_bytes: u64,
        pub total_bytes: u64,
        pub bytes_per_second: u64,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadCompletedEvent {
        pub video: ResolvedVideo,
    }

    #[derive(Debug, Clone)]
    pub enum PlaylistDownloadEvent {
        Started(PlaylistDownloadStartedEvent),
        Completed(PlaylistDownloadCompletedEvent),
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadStartedEvent {
        pub playlist: PartiallyResolvedPlaylist,
    }

    pub struct PlaylistDownloadProgressUpdatedEvent {
        pub video: ResolvedVideo,

        pub completed_videos: u64,
        pub total_videos: u64,
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadCompletedEvent {
        pub playlist: ResolvedPlaylist,
    }

    #[derive(Debug, Clone)]
    pub struct DiagnosticEvent {
        pub level: DiagnosticLevel,
        pub message: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub enum DiagnosticLevel {
        Warning,
        Error,
    }
}

pub mod descriptors {
    use ::domain::{ChannelId, ChannelMetadata, PlaylistId, PlaylistMetadata, VideoId, VideoMetadata};

    use crate::utils::aliases::{MaybeOwnedString, MaybeOwnedVec};

    #[derive(Debug, Clone)]
    pub struct UnresolvedVideo {
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedVideo {
        pub url: MaybeOwnedString,

        pub id: VideoId,
        pub metadata: VideoMetadata,
    }

    pub type ResolvedVideo = ::domain::Video;

    #[derive(Debug, Clone)]
    pub struct UnresolvedPlaylist {
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedPlaylist {
        pub url: MaybeOwnedString,

        pub id: PlaylistId,
        pub metadata: PlaylistMetadata,
        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,
    }

    pub type ResolvedPlaylist = ::domain::Playlist;

    #[derive(Debug, Clone)]
    pub struct UnresolvedChannel {
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedChannel {
        pub url: MaybeOwnedString,

        pub id: ChannelId,
        pub metadata: ChannelMetadata,
        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,
        pub playlists: Option<MaybeOwnedVec<UnresolvedPlaylist>>,
    }

    pub type ResolvedChannel = ::domain::Channel;
}
