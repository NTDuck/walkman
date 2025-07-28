use async_trait::async_trait;
use derive_new::new;
use use_cases::{gateways::PostProcessor, models::descriptors::{ResolvedPlaylist, ResolvedVideo}};

use crate::utils::aliases::Fallible;

#[derive(new)]
pub struct Id3MetadataWriter {
    configurations: Id3MetadataWriterConfigurations,
}

pub struct Id3MetadataWriterConfigurations {
    pub policy: AlbumNamingPolicy,
}

pub enum AlbumNamingPolicy {
    UseVideoAlbum,
    UsePlaylistTitle,
}

#[async_trait]
impl PostProcessor<ResolvedVideo<'_>> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        self.write(video, None)
    }
}

#[async_trait]
impl PostProcessor<ResolvedPlaylist<'_>> for Id3MetadataWriter {
    async fn process(self: ::std::sync::Arc<Self>, playlist: &ResolvedPlaylist) -> Fallible<()> {
        use ::rayon::iter::ParallelIterator as _;
        use ::rayon::iter::IntoParallelIterator as _;

        playlist.videos
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

        video.metadata.title
            .as_deref()
            .map(|title| tag.set_title(title));

        match self.configurations.policy {
            AlbumNamingPolicy::UseVideoAlbum => {
                video.metadata.album
                    .as_deref()
                    .map(|album| tag.set_album(album));
            },

            AlbumNamingPolicy::UsePlaylistTitle => {
                playlist
                    .and_then(|playlist| playlist.metadata.title.as_deref())
                    .map(|title| tag.set_album(title));
            },
        }

        video.metadata.artists
            .as_deref()
            .map(|artists| artists.join(MULTIVALUE_DELIMITER))
            .map(|artists| tag.set_artist(artists));

        video.metadata.genres
            .as_deref()
            .map(|genres| tag.set_genre(genres.join(MULTIVALUE_DELIMITER)));

        tag.write_to_path(&video.path, ::id3::Version::Id3v23)?;

        Ok(())
    }
}

const MULTIVALUE_DELIMITER: &str = ", ";
