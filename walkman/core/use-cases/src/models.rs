pub mod events {
    use ::domain::VideoId;

    use crate::models::descriptors::PartiallyResolvedPlaylist;
    use crate::models::descriptors::PartiallyResolvedVideo;
    use crate::models::descriptors::ResolvedPlaylist;
    use crate::models::descriptors::ResolvedVideo;
    use crate::utils::aliases::MaybeOwnedString;

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
        ProgressUpdated(PlaylistDownloadProgressUpdatedEvent),
        Completed(PlaylistDownloadCompletedEvent),
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadStartedEvent {
        pub playlist: PartiallyResolvedPlaylist,
    }

    #[derive(Debug, Clone)]
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
    use ::domain::ChannelId;
    use ::domain::ChannelMetadata;
    use ::domain::PlaylistId;
    use ::domain::PlaylistMetadata;
    use ::domain::VideoId;
    use ::domain::VideoMetadata;

    use crate::utils::aliases::MaybeOwnedString;
    use crate::utils::aliases::MaybeOwnedVec;

    #[derive(Debug, Clone)]
    pub struct UnresolvedVideo {
        pub id: VideoId,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedVideo {
        pub id: VideoId,
        pub url: MaybeOwnedString,

        pub metadata: VideoMetadata,
    }

    pub type ResolvedVideo = ::domain::Video;

    #[derive(Debug, Clone)]
    pub struct UnresolvedPlaylist {
        pub id: PlaylistId,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedPlaylist {
        pub id: PlaylistId,
        pub url: MaybeOwnedString,

        pub metadata: PlaylistMetadata,

        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,
    }

    pub type ResolvedPlaylist = ::domain::Playlist;

    #[derive(Debug, Clone)]
    pub struct UnresolvedChannel {
        pub id: ChannelId,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedChannel {
        pub id: ChannelId,
        pub url: MaybeOwnedString,

        pub metadata: ChannelMetadata,
        
        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,
        pub playlists: Option<MaybeOwnedVec<UnresolvedPlaylist>>,
    }

    pub type ResolvedChannel = ::domain::Channel;
}
