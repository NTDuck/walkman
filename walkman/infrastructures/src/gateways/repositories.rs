use ::async_trait::async_trait;
use ::use_cases::gateways::UrlRepository;
use ::futures::prelude::*;

use crate::utils::aliases::{BoxedStream, Fallible, MaybeOwnedPath, MaybeOwnedString};

pub struct FilesystemResourcesRepository {
    pub videos_path: MaybeOwnedPath,
    pub playlists_path: MaybeOwnedPath,
}

#[async_trait]
impl UrlRepository for FilesystemResourcesRepository {
    async fn insert_video_url(self: ::std::sync::Arc<Self>, url: MaybeOwnedString) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.videos_path)
            .await?;

        file.write_all(format!("{}\n", url).as_bytes()).await?;

        Ok(())
    }

    async fn insert_playlist_url(self: ::std::sync::Arc<Self>, url: MaybeOwnedString) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.playlists_path)
            .await?;

        file.write_all(format!("{}\n", url).as_bytes()).await?;

        Ok(())
    }

    async fn get_urls(self: ::std::sync::Arc<Self>) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)> {
        use ::tokio::io::AsyncBufReadExt as _;

        let (video_urls_tx, video_urls_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlist_urls_tx, playlist_urls_rx) = ::tokio::sync::mpsc::unbounded_channel();

        match ::tokio::fs::File::open(&self.videos_path).await {
            Ok(file) => {
                ::tokio::spawn(async move {
                    let lines = ::tokio::io::BufReader::new(file).lines();

                    ::tokio_stream::wrappers::LinesStream::new(lines)
                        .filter_map(|line| async move { line.ok() })
                        .map(|line| line.to_owned().into())
                        .map(Ok)
                        .try_for_each(|url| async { video_urls_tx.send(url) })
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
                        .map(Ok)
                        .try_for_each(|url| async { playlist_urls_tx.send(url) })
                        .await
                });
            }

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => {},
            Err(err) => return Err(err.into()),
        }

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_urls_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_urls_rx)),
        ))
    }
}
