use ::async_trait::async_trait;
use ::domain::{ChannelUrl, PlaylistUrl, VideoUrl};
use ::use_cases::gateways::Insert;
use ::use_cases::gateways::UrlRepository;
use ::futures::prelude::*;

use crate::utils::aliases::{BoxedStream, Fallible, MaybeOwnedPath, MaybeOwnedString};

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct FilesystemResourcesRepository {
    video_urls_path: MaybeOwnedPath,
    playlist_urls_path: MaybeOwnedPath,
    channel_urls_path: MaybeOwnedPath,
}

#[async_trait]
impl UrlRepository for FilesystemResourcesRepository {
    async fn values(self: ::std::sync::Arc<Self>) -> Fallible<(BoxedStream<VideoUrl>, BoxedStream<PlaylistUrl>, BoxedStream<ChannelUrl>)> {
        let (video_urls, playlist_urls, channel_urls) = ::tokio::try_join!(
            ::std::sync::Arc::clone(&self).get(),
            ::std::sync::Arc::clone(&self).get(),
            ::std::sync::Arc::clone(&self).get(),
        )?;

        Ok((video_urls, playlist_urls, channel_urls))
    }
}

#[async_trait]
impl Insert<VideoUrl> for FilesystemResourcesRepository {
    async fn insert(self: ::std::sync::Arc<Self>, url: VideoUrl) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.video_urls_path)
            .await?;

        file.write_all(format!("{}\n", *url).as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
impl Insert<PlaylistUrl> for FilesystemResourcesRepository {
    async fn insert(self: ::std::sync::Arc<Self>, url: PlaylistUrl) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.playlist_urls_path)
            .await?;

        file.write_all(format!("{}\n", *url).as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
impl Insert<ChannelUrl> for FilesystemResourcesRepository {
    async fn insert(self: ::std::sync::Arc<Self>, url: ChannelUrl) -> Fallible<()> {
        use ::tokio::io::AsyncWriteExt as _;

        let mut file = ::tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.channel_urls_path)
            .await?;

        file.write_all(format!("{}\n", *url).as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
trait Get<Item>: ::core::marker::Send + ::core::marker::Sync {
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<Item>;
}

#[async_trait]
impl Get<BoxedStream<VideoUrl>> for FilesystemResourcesRepository {
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<VideoUrl>> {
        use ::tokio::io::AsyncBufReadExt as _;

        match ::tokio::fs::File::open(&self.video_urls_path).await {
            Ok(file) => {
                let lines = ::tokio::io::BufReader::new(file).lines();

                let urls = ::tokio_stream::wrappers::LinesStream::new(lines)
                    .filter_map(|line| async move { line.ok() })
                    .map(Into::<MaybeOwnedString>::into)
                    .map(Into::<VideoUrl>::into);

                Ok(::std::boxed::Box::pin(urls))
            },

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => Ok(::std::boxed::Box::pin(::futures::stream::empty())),
            Err(err) => Err(err.into()),
        }
    }
}

#[async_trait]
impl Get<BoxedStream<PlaylistUrl>> for FilesystemResourcesRepository {
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<PlaylistUrl>> {
        use ::tokio::io::AsyncBufReadExt as _;

        match ::tokio::fs::File::open(&self.playlist_urls_path).await {
            Ok(file) => {
                let lines = ::tokio::io::BufReader::new(file).lines();

                let urls = ::tokio_stream::wrappers::LinesStream::new(lines)
                    .filter_map(|line| async move { line.ok() })
                    .map(Into::<MaybeOwnedString>::into)
                    .map(Into::<PlaylistUrl>::into);

                Ok(::std::boxed::Box::pin(urls))
            },

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => Ok(::std::boxed::Box::pin(::futures::stream::empty())),
            Err(err) => Err(err.into()),
        }
    }
}

#[async_trait]
impl Get<BoxedStream<ChannelUrl>> for FilesystemResourcesRepository {
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<ChannelUrl>> {
        use ::tokio::io::AsyncBufReadExt as _;

        match ::tokio::fs::File::open(&self.channel_urls_path).await {
            Ok(file) => {
                let lines = ::tokio::io::BufReader::new(file).lines();

                let urls = ::tokio_stream::wrappers::LinesStream::new(lines)
                    .filter_map(|line| async move { line.ok() })
                    .map(Into::<MaybeOwnedString>::into)
                    .map(Into::<ChannelUrl>::into);

                Ok(::std::boxed::Box::pin(urls))
            },

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => Ok(::std::boxed::Box::pin(::futures::stream::empty())),
            Err(err) => Err(err.into()),
        }
    }
}
