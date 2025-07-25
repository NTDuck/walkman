pub mod events {
    use crate::{models::descriptors::{PartiallyResolvedPlaylist, PartiallyResolvedVideo, ResolvedPlaylist, ResolvedVideo}, utils::aliases::MaybeOwnedString};

    #[derive(Debug, Clone)]
    pub struct Event<Payload> {
        pub metadata: EventMetadata,
        pub payload: Payload,
    }

    #[derive(Debug, Clone)]
    pub struct EventMetadata {
        pub worker_id: MaybeOwnedString,
        pub correlation_id: MaybeOwnedString,
        pub timestamp: ::std::time::SystemTime,
    }

    #[derive(Debug, Clone)]
    pub struct EventRef<'this, Payload> {
        pub metadata: &'this EventMetadata,
        pub payload: &'this Payload,
    }
    
    impl<Payload> Event<Payload> {
        pub fn with_metadata(self, metadata: EventMetadata) -> Self {
            Self {
                metadata,
                ..self
            }
        }
        
        pub fn with_payload<'this, OtherPayload>(&'this self, payload: &'this OtherPayload) -> EventRef<'this, OtherPayload> {
            EventRef {
                metadata: &self.metadata,
                payload,
            }
        }
    }

    pub type VideoDownloadEvent = Event<VideoDownloadEventPayload>;

    #[derive(Debug, Clone)]
    pub enum VideoDownloadEventPayload {
        Started(VideoDownloadStartedEventPayload),
        ProgressUpdated(VideoDownloadProgressUpdatedEventPayload),
        Completed(VideoDownloadCompletedEventPayload),
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadStartedEventPayload {
        pub video: PartiallyResolvedVideo,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadProgressUpdatedEventPayload {
        pub percentage: u8,

        pub eta: MaybeOwnedString,
        pub size: MaybeOwnedString,
        pub speed: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadCompletedEventPayload {
        pub video: ResolvedVideo,
    }

    pub type PlaylistDownloadEvent = Event<PlaylistDownloadEventPayload>;

    #[derive(Debug, Clone)]
    pub enum PlaylistDownloadEventPayload {
        Started(PlaylistDownloadStartedEventPayload),
        ProgressUpdated(PlaylistDownloadProgressUpdatedEventPayload),
        Completed(PlaylistDownloadCompletedEventPayload),
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadStartedEventPayload {
        pub playlist: PartiallyResolvedPlaylist,
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadProgressUpdatedEventPayload {
        pub video: ResolvedVideo,

        pub completed: usize,
        pub total: usize,
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadCompletedEventPayload {
        pub playlist: ResolvedPlaylist,
    }

    pub type DiagnosticEvent = Event<DiagnosticEventPayload>;

    #[derive(Debug, Clone)]
    pub struct DiagnosticEventPayload {
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
        pub videos: Option<Vec<UnresolvedVideo>>,
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
        pub playlists: Option<Vec<UnresolvedPlaylist>>,
        pub videos: Option<Vec<UnresolvedVideo>>,
    }

    pub type ResolvedChannel = ::domain::Channel;
}
