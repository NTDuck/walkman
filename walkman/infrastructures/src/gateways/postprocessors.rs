use ::async_trait::async_trait;
use ::use_cases::gateways::PostProcessor;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::rayon::prelude::*;

use crate::utils::aliases::Fallible;

pub struct Id3MetadataWriter {
    pub policy: AlbumNamingPolicy,
}

pub enum AlbumNamingPolicy {
    UseVideoAlbum,
    UsePlaylistTitle,
}

#[async_trait]
impl PostProcessor<ResolvedVideo> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        self.write(video, None)
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
            .try_for_each(|video| ::std::sync::Arc::clone(&self).write(video, Some(playlist)))
    }
}

impl Id3MetadataWriter {
    fn write(self: ::std::sync::Arc<Self>, video: &ResolvedVideo, playlist: Option<&ResolvedPlaylist>) -> Fallible<()> {
        use ::id3::TagLike as _;

        let mut tag = ::id3::Tag::new();

        if let Some(title) = video.metadata.title.as_deref() {
            tag.set_title(title)
        }

        match self.policy {
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

        if let Some(artists) = video.metadata.artists.as_deref().map(|artists| artists.join(", ")) {
            tag.set_artist(artists)
        }

        if let Some(genres) = video.metadata.genres.as_deref() {
            tag.set_genre(genres.join(", "))
        }

        tag.write_to_path(&video.path, ::id3::Version::Id3v23)?;

        Ok(())
    }
}
