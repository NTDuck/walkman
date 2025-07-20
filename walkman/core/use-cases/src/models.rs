pub mod events {
    use crate::{models::descriptors::{PartiallyResolvedPlaylist, PartiallyResolvedVideo, ResolvedPlaylist, ResolvedVideo}, utils::aliases::MaybeOwnedString};

    pub struct Event<Payload> {
        pub metadata: EventMetadata,
        pub payload: Payload,
    }

    pub struct EventMetadata {
        pub worker_id: MaybeOwnedString,
        pub correlation_id: MaybeOwnedString,
        pub timestamp: ::std::time::SystemTime,
    }

    pub type VideoDownloadEvent = Event<VideoDownloadEventPayload>;

    pub enum VideoDownloadEventPayload {
        Started(VideoDownloadStartedEventPayload),
        ProgressUpdated(VideoDownloadProgressUpdatedEventPayload),
        Completed(VideoDownloadCompletedEventPayload),
    }

    pub struct VideoDownloadStartedEventPayload {
        pub video: PartiallyResolvedVideo,
    }

    pub struct VideoDownloadProgressUpdatedEventPayload {
        pub percentage: u8,

        pub eta: MaybeOwnedString,
        pub size: MaybeOwnedString,
        pub speed: MaybeOwnedString,
    }

    pub struct VideoDownloadCompletedEventPayload {
        pub video: ResolvedVideo,
    }

    pub type PlaylistDownloadEvent = Event<PlaylistDownloadEventPayload>;

    pub enum PlaylistDownloadEventPayload {
        Started(PlaylistDownloadStartedEventPayload),
        ProgressUpdated(PlaylistDownloadProgressUpdatedEventPayload),
        Completed(PlaylistDownloadCompletedEventPayload),
    }

    pub struct PlaylistDownloadStartedEventPayload {
        pub playlist: PartiallyResolvedPlaylist,
    }

    pub struct PlaylistDownloadProgressUpdatedEventPayload {
        pub video: ResolvedVideo,

        pub completed: usize,
        pub total: usize,
    }

    pub struct PlaylistDownloadCompletedEventPayload {
        pub playlist: ResolvedPlaylist,
    }

    pub type DiagnosticEvent = Event<DiagnosticEventPayload>;

    pub struct DiagnosticEventPayload {
        pub level: DiagnosticLevel,
        pub message: MaybeOwnedString,
    }

    pub enum DiagnosticLevel {
        Warning,
        Error,
    }
}

pub mod descriptors {
    use ::domain::{ChannelId, ChannelMetadata, PlaylistId, PlaylistMetadata, VideoId, VideoMetadata};

    use crate::utils::aliases::MaybeOwnedString;

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
        pub videos: Vec<UnresolvedVideo>,
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
        pub videos: Vec<UnresolvedVideo>,
        pub playlists: Vec<UnresolvedPlaylist>,
    }

    pub type ResolvedChannel = ::domain::Channel;
}
