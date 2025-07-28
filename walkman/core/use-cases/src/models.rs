pub mod events {
    use domain::VideoId;

    use crate::{models::descriptors::{PartiallyResolvedPlaylist, PartiallyResolvedVideo, ResolvedPlaylist, ResolvedVideo}, utils::aliases::MaybeOwnedString};

    #[derive(Debug, Clone)]
    pub enum VideoDownloadEvent<'a> {
        Started(VideoDownloadStartedEvent<'a>),
        ProgressUpdated(VideoDownloadProgressUpdatedEvent<'a>),
        Completed(VideoDownloadCompletedEvent<'a>),
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadStartedEvent<'a> {
        pub video: PartiallyResolvedVideo<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadProgressUpdatedEvent<'a> {
        pub id: VideoId<'a>,

        pub eta: ::std::time::Duration,
        pub elapsed: ::std::time::Duration,

        pub downloaded_bytes: u64,
        pub total_bytes: u64,
        pub bytes_per_second: u64,
    }

    #[derive(Debug, Clone)]
    pub struct VideoDownloadCompletedEvent<'a> {
        pub video: ResolvedVideo<'a>,
    }

    #[derive(Debug, Clone)]
    pub enum PlaylistDownloadEvent<'a> {
        Started(PlaylistDownloadStartedEvent<'a>),
        Completed(PlaylistDownloadCompletedEvent<'a>),
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadStartedEvent<'a> {
        pub playlist: PartiallyResolvedPlaylist<'a>,
    }

    pub struct PlaylistDownloadProgressUpdatedEvent<'a> {
        pub video: ResolvedVideo<'a>,

        pub completed_videos: u64,
        pub total_videos: u64,
    }

    #[derive(Debug, Clone)]
    pub struct PlaylistDownloadCompletedEvent<'a> {
        pub playlist: ResolvedPlaylist<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct DiagnosticEvent<'a> {
        pub level: DiagnosticLevel,
        pub message: MaybeOwnedString<'a>,
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
    pub struct UnresolvedVideo<'a> {
        pub url: MaybeOwnedString<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedVideo<'a> {
        pub url: MaybeOwnedString<'a>,

        pub id: VideoId<'a>,
        pub metadata: VideoMetadata<'a>,
    }

    pub type ResolvedVideo<'a> = ::domain::Video<'a>;

    #[derive(Debug, Clone)]
    pub struct UnresolvedPlaylist<'a> {
        pub url: MaybeOwnedString<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedPlaylist<'a> {
        pub url: MaybeOwnedString<'a>,

        pub id: PlaylistId<'a>,
        pub metadata: PlaylistMetadata<'a>,
        pub videos: Option<MaybeOwnedVec<'a, UnresolvedVideo<'a>>>,
    }

    pub type ResolvedPlaylist<'a> = ::domain::Playlist<'a>;

    #[derive(Debug, Clone)]
    pub struct UnresolvedChannel<'a> {
        pub url: MaybeOwnedString<'a>,
    }

    #[derive(Debug, Clone)]
    pub struct PartiallyResolvedChannel<'a> {
        pub url: MaybeOwnedString<'a>,

        pub id: ChannelId<'a>,
        pub metadata: ChannelMetadata<'a>,
        pub videos: Option<MaybeOwnedVec<'a, UnresolvedVideo<'a>>>,
        pub playlists: Option<MaybeOwnedVec<'a, UnresolvedPlaylist<'a>>>,
    }

    pub type ResolvedChannel<'a> = ::domain::Channel<'a>;
}
