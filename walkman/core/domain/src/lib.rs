pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;

#[derive(Debug, Clone)]
pub struct Video {
    pub id: VideoId,
    pub url: VideoUrl,

    pub metadata: VideoMetadata,

    pub path: VideoFilePath,
}

#[derive(Debug, Clone)]
pub struct VideoId(MaybeOwnedString);

impl ::std::ops::Deref for VideoId {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VideoId> for MaybeOwnedString {
    fn from(outer: VideoId) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for VideoId {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct VideoUrl(MaybeOwnedString);

impl ::std::ops::Deref for VideoUrl {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VideoUrl> for MaybeOwnedString {
    fn from(outer: VideoUrl) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for VideoUrl {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub title: Option<MaybeOwnedString>,
    pub album: Option<MaybeOwnedString>,
    pub artists: Option<MaybeOwnedVec<MaybeOwnedString>>,
    pub genres: Option<MaybeOwnedVec<MaybeOwnedString>>,
}

#[derive(Debug, Clone)]
pub struct VideoFilePath(MaybeOwnedPath);

impl ::std::ops::Deref for VideoFilePath {
    type Target = MaybeOwnedPath;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VideoFilePath> for MaybeOwnedPath {
    fn from(outer: VideoFilePath) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedPath> for VideoFilePath {
    fn from(inner: MaybeOwnedPath) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub id: PlaylistId,
    pub url: PlaylistUrl,

    pub metadata: PlaylistMetadata,

    pub videos: Option<MaybeOwnedVec<Video>>,
}

#[derive(Debug, Clone)]
pub struct PlaylistId(MaybeOwnedString);

impl ::std::ops::Deref for PlaylistId {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PlaylistId> for MaybeOwnedString {
    fn from(outer: PlaylistId) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for PlaylistId {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistUrl(MaybeOwnedString);

impl ::std::ops::Deref for PlaylistUrl {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PlaylistUrl> for MaybeOwnedString {
    fn from(outer: PlaylistUrl) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for PlaylistUrl {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct PlaylistMetadata {
    pub title: Option<MaybeOwnedString>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: ChannelId,
    pub url: ChannelUrl,

    pub metadata: ChannelMetadata,
    
    pub videos: Option<MaybeOwnedVec<Video>>,
    pub playlists: Option<MaybeOwnedVec<Playlist>>,
}

#[derive(Debug, Clone)]
pub struct ChannelId(MaybeOwnedString);

impl ::std::ops::Deref for ChannelId {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ChannelId> for MaybeOwnedString {
    fn from(outer: ChannelId) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for ChannelId {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct ChannelUrl(MaybeOwnedString);

impl ::std::ops::Deref for ChannelUrl {
    type Target = MaybeOwnedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ChannelUrl> for MaybeOwnedString {
    fn from(outer: ChannelUrl) -> Self {
        outer.0
    }
}

impl From<MaybeOwnedString> for ChannelUrl {
    fn from(inner: MaybeOwnedString) -> Self {
        Self(inner)
    }
}

#[derive(Debug, Clone)]
pub struct ChannelMetadata {
    pub title: Option<MaybeOwnedString>,
}
