pub mod events {
    use crate::models::descriptors::PartiallyResolvedChannel;
    use crate::models::descriptors::PartiallyResolvedPlaylist;
    use crate::models::descriptors::PartiallyResolvedVideo;
    use crate::models::descriptors::ResolvedChannel;
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
    #[derive(::bon::Builder)]
    pub struct VideoDownloadStartedEvent {
        pub video: PartiallyResolvedVideo,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct VideoDownloadProgressUpdatedEvent {
        pub video_id: MaybeOwnedString,

        pub eta: ::std::time::Duration,
        pub elapsed: ::std::time::Duration,

        pub downloaded_bytes: u64,
        pub total_bytes: u64,
        pub bytes_per_second: u64,

        #[builder(skip = ((downloaded_bytes as f64) / (total_bytes as f64) * 100.0) as u64)]
        pub percentage: u64,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
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
    #[derive(::bon::Builder)]
    pub struct PlaylistDownloadStartedEvent {
        pub playlist: PartiallyResolvedPlaylist,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PlaylistDownloadProgressUpdatedEvent {
        pub playlist_id: MaybeOwnedString,

        pub completed_videos: u64,
        pub total_videos: u64,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PlaylistDownloadCompletedEvent {
        pub playlist: ResolvedPlaylist,
    }

    #[derive(Debug, Clone)]
    pub enum ChannelDownloadEvent {
        Started(ChannelDownloadStartedEvent),
        ProgressUpdated(ChannelDownloadProgressUpdatedEvent),
        Completed(ChannelDownloadCompletedEvent),
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ChannelDownloadStartedEvent {
        pub channel: PartiallyResolvedChannel,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ChannelDownloadProgressUpdatedEvent {
        pub channel_id: MaybeOwnedString,

        pub completed_videos: u64,
        pub total_videos: u64,

        pub completed_playlists: u64,
        pub total_playlists: u64,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ChannelDownloadCompletedEvent {
        pub channel: ResolvedChannel,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
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
    use crate::utils::aliases::MaybeOwnedPath;
    use crate::utils::aliases::MaybeOwnedString;
    use crate::utils::aliases::MaybeOwnedVec;

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct UnresolvedVideo {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PartiallyResolvedVideo {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: VideoMetadata,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ResolvedVideo {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: VideoMetadata,

        pub path: MaybeOwnedPath,
    }

    impl From<::domain::Video> for ResolvedVideo {
        fn from(this: ::domain::Video) -> Self {
            Self {
                id: this.id.into(),
                url: this.url.into(),
                metadata: this.metadata.into(),
                path: this.path.into(),
            }
        }
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct VideoMetadata {
        #[builder(required)]
        pub title: Option<MaybeOwnedString>,

        #[builder(required)]
        pub album: Option<MaybeOwnedString>,

        #[builder(required)]
        pub artists: Option<MaybeOwnedVec<MaybeOwnedString>>,

        #[builder(required)]
        pub genres: Option<MaybeOwnedVec<MaybeOwnedString>>,
    }

    impl From<::domain::VideoMetadata> for VideoMetadata {
        fn from(this: ::domain::VideoMetadata) -> Self {
            Self {
                title: this.title,
                album: this.album,
                artists: this.artists,
                genres: this.genres,
            }
        }
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct UnresolvedPlaylist {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PartiallyResolvedPlaylist {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: PlaylistMetadata,

        #[builder(required)]
        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ResolvedPlaylist {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: PlaylistMetadata,

        #[builder(required)]
        pub videos: Option<MaybeOwnedVec<ResolvedVideo>>,
    }

    impl From<::domain::Playlist> for ResolvedPlaylist {
        fn from(this: ::domain::Playlist) -> Self {
            Self {
                id: this.id.into(),
                url: this.url.into(),
                metadata: this.metadata.into(),
                videos: this
                    .videos
                    .map(|videos| videos.iter().cloned().map(Into::into).collect::<Vec<_>>().into()),
            }
        }
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PlaylistMetadata {
        #[builder(required)]
        pub title: Option<MaybeOwnedString>,
    }

    impl From<::domain::PlaylistMetadata> for PlaylistMetadata {
        fn from(this: ::domain::PlaylistMetadata) -> Self {
            Self { title: this.title }
        }
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct UnresolvedChannel {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct PartiallyResolvedChannel {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: ChannelMetadata,

        #[builder(required)]
        pub videos: Option<MaybeOwnedVec<UnresolvedVideo>>,

        #[builder(required)]
        pub playlists: Option<MaybeOwnedVec<UnresolvedPlaylist>>,
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ResolvedChannel {
        pub id: MaybeOwnedString,
        pub url: MaybeOwnedString,

        pub metadata: ChannelMetadata,

        #[builder(required)]
        pub videos: Option<MaybeOwnedVec<ResolvedVideo>>,

        #[builder(required)]
        pub playlists: Option<MaybeOwnedVec<ResolvedPlaylist>>,
    }

    impl From<::domain::Channel> for ResolvedChannel {
        fn from(this: ::domain::Channel) -> Self {
            Self {
                id: this.id.into(),
                url: this.url.into(),
                metadata: this.metadata.into(),
                videos: this
                    .videos
                    .map(|videos| videos.iter().cloned().map(Into::into).collect::<Vec<_>>().into()),
                playlists: this
                    .playlists
                    .map(|playlists| playlists.iter().cloned().map(Into::into).collect::<Vec<_>>().into()),
            }
        }
    }

    #[derive(Debug, Clone)]
    #[derive(::bon::Builder)]
    pub struct ChannelMetadata {
        #[builder(required)]
        pub title: Option<MaybeOwnedString>,
    }

    impl From<::domain::ChannelMetadata> for ChannelMetadata {
        fn from(this: ::domain::ChannelMetadata) -> Self {
            Self { title: this.title }
        }
    }
}
