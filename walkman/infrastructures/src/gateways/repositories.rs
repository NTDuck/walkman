use ::async_trait::async_trait;
use ::use_cases::{gateways::{Insert, ResourceRepository}, models::descriptors::{UnresolvedPlaylist, UnresolvedVideo}};
use ::futures::prelude::*;

use crate::utils::aliases::{BoxedStream, Fallible, MaybeOwnedPath};

pub struct FilesystemResourcesRepository {
    pub videos_path: MaybeOwnedPath,
    pub playlists_path: MaybeOwnedPath,
}

#[async_trait]
impl ResourceRepository for FilesystemResourcesRepository {
    async fn get_all(self: ::std::sync::Arc<Self>) -> Fallible<(BoxedStream<UnresolvedVideo>, BoxedStream<UnresolvedPlaylist>)> {
        use ::tokio::io::AsyncBufReadExt as _;

        let (videos_tx, videos_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlists_tx, playlists_rx) = ::tokio::sync::mpsc::unbounded_channel();

        match ::tokio::fs::File::open(&self.videos_path).await {
            Ok(file) => {
                ::tokio::spawn(async move {
                    let lines = ::tokio::io::BufReader::new(file).lines();

                    ::tokio_stream::wrappers::LinesStream::new(lines)
                        .filter_map(|line| async move { line.ok() })
                        .map(|line| line.to_owned().into())
                        .map(|url| UnresolvedVideo { url })
                        .map(Ok)
                        .try_for_each(|video| async { videos_tx.send(video) })
                        .await
                });
            }

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => {},
            Err(err) => return Err(err.into()),
        }

        match ::tokio::fs::File::open(&self.playlists_path).await {
            Ok(file) => {
                ::tokio::spawn(async move {
                    let lines = ::tokio::io::BufReader::new(file).lines();

                    ::tokio_stream::wrappers::LinesStream::new(lines)
                        .filter_map(|line| async move { line.ok() })
                        .map(|line| line.to_owned().into())
                        .map(|url| UnresolvedPlaylist { url })
                        .map(Ok)
                        .try_for_each(|playlist| async { playlists_tx.send(playlist) })
                        .await
                });
            }

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => {},
            Err(err) => return Err(err.into()),
        }

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(videos_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlists_rx)),
        ))
    }
}

#[async_trait]
impl Insert<UnresolvedVideo> for FilesystemResourcesRepository {
    async fn insert(self: ::std::sync::Arc<Self>, video: UnresolvedVideo) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.videos_path)
            .await?;

        file.write_all(format!("{}\n", video.url).as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
impl Insert<UnresolvedPlaylist> for FilesystemResourcesRepository {
    async fn insert(self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.playlists_path)
            .await?;

        file.write_all(format!("{}\n", playlist.url).as_bytes()).await?;

        Ok(())
    }
}
