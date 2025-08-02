use ::async_trait::async_trait;
use ::use_cases::gateways::PostProcessor;
use ::use_cases::models::descriptors::ResolvedChannel;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::rayon::prelude::*;

use crate::utils::aliases::Fallible;

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct Id3MetadataWriter {
    album_naming_policy: AlbumNamingPolicy,
    artists_naming_policy: ArtistsNamingPolicy,
}

pub enum AlbumNamingPolicy {
    UseVideoAlbum,
    UsePlaylistTitle,
}

pub enum ArtistsNamingPolicy {
    UseOnlyVideoArtists,
    UseOnlyChannelTitle,
    UseBothVideoArtistsAndChannelTitle,
}

#[async_trait]
impl PostProcessor<ResolvedVideo> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        self.write()
            .video(video)
            .call()
    }
}

#[async_trait]
impl PostProcessor<ResolvedPlaylist> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, playlist: &ResolvedPlaylist) -> Fallible<()> {
        playlist
            .videos
            .as_deref()
            .into_par_iter()
            .flatten()
            .try_for_each(|video| ::std::sync::Arc::clone(&self).write()
                .video(video)
                .playlist(playlist)
                .call())
    }
}

#[async_trait]
impl PostProcessor<ResolvedChannel> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, _: &ResolvedChannel) -> Fallible<()> {
        todo!()
    }
}

#[::bon::bon]
impl Id3MetadataWriter {
    #[builder]
    fn write(self: ::std::sync::Arc<Self>, video: &ResolvedVideo, playlist: Option<&ResolvedPlaylist>, channel: Option<&ResolvedChannel>) -> Fallible<()> {
        use ::id3::TagLike as _;

        let mut tag = ::id3::Tag::new();

        if let Some(title) = video.metadata.title.as_deref() {
            tag.set_title(title)
        }

        match self.album_naming_policy {
            AlbumNamingPolicy::UseVideoAlbum =>
                if let Some(album) = video.metadata.album.as_deref() {
                    tag.set_album(album)
                },
            AlbumNamingPolicy::UsePlaylistTitle => {
                if let Some(title) = playlist.and_then(|playlist| playlist.metadata.title.as_deref()) {
                    tag.set_album(title)
                }
            },
        }

        match self.artists_naming_policy {
            ArtistsNamingPolicy::UseOnlyVideoArtists => {
                if let Some(artists) = video.metadata.artists.as_deref().map(|artists| artists.join(", ")) {
                    tag.set_artist(artists)
                }
            },
            ArtistsNamingPolicy::UseOnlyChannelTitle => {
                if let Some(title) = channel.and_then(|channel| channel.metadata.title.as_deref()) {
                    tag.set_artist(title)
                }
            },
            ArtistsNamingPolicy::UseBothVideoArtistsAndChannelTitle => {
                match (video.metadata.artists.as_deref().map(|artists| artists.join(", ")), channel.and_then(|channel| channel.metadata.title.as_deref())) {
                    (Some(artists), Some(title)) => tag.set_artist(format!("{}, {}", artists, title)),
                    (Some(artists), None) => tag.set_artist(artists),
                    (None, Some(title)) => tag.set_artist(title),
                    (None, None) => (),
                }
            },
        }

        if let Some(genres) = video.metadata.genres.as_deref() {
            tag.set_genre(genres.join(", "))
        }

        tag.write_to_path(&video.path, ::id3::Version::Id3v23)?;

        Ok(())
    }
}
