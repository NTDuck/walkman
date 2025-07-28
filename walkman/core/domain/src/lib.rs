pub(crate) mod utils;

use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;

#[derive(Debug, Clone)]
pub struct Video<'a> {
    pub url: MaybeOwnedString<'a>,

    pub id: VideoId<'a>,
    pub metadata: VideoMetadata<'a>,

    pub path: MaybeOwnedPath<'a>,
}

pub type VideoId<'a> = MaybeOwnedString<'a>;

#[derive(Debug, Clone)]
pub struct VideoMetadata<'a> {
    pub title: Option<MaybeOwnedString<'a>>,
    pub album: Option<MaybeOwnedString<'a>>,
    pub artists: Option<MaybeOwnedVec<'a, MaybeOwnedString<'a>>>,
    pub genres: Option<MaybeOwnedVec<'a, MaybeOwnedString<'a>>>,
}

#[derive(Debug, Clone)]
pub struct Playlist<'a> {
    pub url: MaybeOwnedString<'a>,

    pub id: PlaylistId<'a>,
    pub metadata: PlaylistMetadata<'a>,
    pub videos: Option<MaybeOwnedVec<'a, Video<'a>>>,
}

pub type PlaylistId<'a> = MaybeOwnedString<'a>;

#[derive(Debug, Clone)]
pub struct PlaylistMetadata<'a> {
    pub title: Option<MaybeOwnedString<'a>>,
}

#[derive(Debug, Clone)]
pub struct Channel<'a> {
    pub url: MaybeOwnedString<'a>,

    pub id: ChannelId<'a>,
    pub metadata: ChannelMetadata<'a>,
    pub videos: Option<MaybeOwnedVec<'a, Video<'a>>>,
    pub playlists: Option<MaybeOwnedVec<'a, Playlist<'a>>>,
}

pub type ChannelId<'a> = MaybeOwnedString<'a>;

#[derive(Debug, Clone)]
pub struct ChannelMetadata<'a> {
    pub title: Option<MaybeOwnedString<'a>>,
}
