use ::async_trait::async_trait;
use ::domain::ChannelUrl;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;
use ::futures::prelude::*;
use ::use_cases::gateways::Insert;
use ::use_cases::gateways::UrlRepository;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct FilesystemResourcesRepository {
    video_urls_path: MaybeOwnedPath,
    playlist_urls_path: MaybeOwnedPath,
    channel_urls_path: MaybeOwnedPath,
}

#[async_trait]
impl UrlRepository for FilesystemResourcesRepository {
    async fn values(
        self: ::std::sync::Arc<Self>,
    ) -> Fallible<(BoxedStream<VideoUrl>, BoxedStream<PlaylistUrl>, BoxedStream<ChannelUrl>)> {
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

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound =>
                Ok(::std::boxed::Box::pin(::futures::stream::empty())),
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

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound =>
                Ok(::std::boxed::Box::pin(::futures::stream::empty())),
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

            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound =>
                Ok(::std::boxed::Box::pin(::futures::stream::empty())),
            Err(err) => Err(err.into()),
        }
    }
}

#[derive(::bon::Builder)]
#[builder(on(_, into), finish_fn(name = _build, vis = "pub(self)"))]
pub struct EncryptedFilesystemResourcesRepository {
    #[builder(field = unreachable!())]
    video_urls_file: ::tokio::sync::RwLock<::tokio::fs::File>,

    #[builder(field = unreachable!())]
    playlist_urls_file: ::tokio::sync::RwLock<::tokio::fs::File>,

    #[builder(field = unreachable!())]
    channel_urls_file: ::tokio::sync::RwLock<::tokio::fs::File>,

    #[builder(getter(vis = "pub(self)"))]
    video_urls_path: MaybeOwnedPath,

    #[builder(getter(vis = "pub(self)"))]
    playlist_urls_path: MaybeOwnedPath,

    #[builder(getter(vis = "pub(self)"))]
    channel_urls_path: MaybeOwnedPath,
}

impl<State> EncryptedFilesystemResourcesRepositoryBuilder<State>
where
    State: encrypted_filesystem_resources_repository_builder::IsComplete,
{
    pub async fn build(self) -> Fallible<EncryptedFilesystemResourcesRepository>
    where
        State::VideoUrlsPath: encrypted_filesystem_resources_repository_builder::IsSet,
        State::PlaylistUrlsPath: encrypted_filesystem_resources_repository_builder::IsSet,
        State::ChannelUrlsPath: encrypted_filesystem_resources_repository_builder::IsSet,
    {
        let video_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.get_video_urls_path())
            .await?;

        let playlist_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.get_playlist_urls_path())
            .await?;

        let channel_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.get_channel_urls_path())
            .await?;

        let mut this = self._build();

        this.video_urls_file = ::tokio::sync::RwLock::new(video_urls_file);
        this.playlist_urls_file = ::tokio::sync::RwLock::new(playlist_urls_file);
        this.channel_urls_file = ::tokio::sync::RwLock::new(channel_urls_file);
        
        Ok(this)
    }
}
