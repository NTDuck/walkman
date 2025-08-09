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
pub struct CompressedSerializedFilesystemResourcesRepository<State = ::std::hash::RandomState> {
    #[builder(field = ::std::mem::MaybeUninit::uninit())]
    video_urls_file: ::std::mem::MaybeUninit<::tokio::sync::Mutex<::tokio::fs::File>>,

    #[builder(field = ::std::mem::MaybeUninit::uninit())]
    playlist_urls_file: ::std::mem::MaybeUninit<::tokio::sync::Mutex<::tokio::fs::File>>,

    #[builder(field = ::std::mem::MaybeUninit::uninit())]
    channel_urls_file: ::std::mem::MaybeUninit<::tokio::sync::Mutex<::tokio::fs::File>>,

    serializer: ::std::sync::Arc<dyn Serializer<::std::collections::HashSet<MaybeOwnedString, State>>>,
    compressor: ::std::sync::Arc<dyn Compressor>,

    #[allow(unused)]
    #[builder(getter(vis = "pub(self)"))]
    video_urls_path: MaybeOwnedPath,

    #[allow(unused)]
    #[builder(getter(vis = "pub(self)"))]
    playlist_urls_path: MaybeOwnedPath,

    #[allow(unused)]
    #[builder(getter(vis = "pub(self)"))]
    channel_urls_path: MaybeOwnedPath,
}

impl<State, BuilderState> CompressedSerializedFilesystemResourcesRepositoryBuilder<State, BuilderState>
where
    BuilderState: compressed_serialized_filesystem_resources_repository_builder::IsComplete,
{
    pub async fn build(self) -> Fallible<CompressedSerializedFilesystemResourcesRepository<State>>
    where
        BuilderState::Serializer: compressed_serialized_filesystem_resources_repository_builder::IsSet,
        BuilderState::Compressor: compressed_serialized_filesystem_resources_repository_builder::IsSet,
        BuilderState::VideoUrlsPath: compressed_serialized_filesystem_resources_repository_builder::IsSet,
        BuilderState::PlaylistUrlsPath: compressed_serialized_filesystem_resources_repository_builder::IsSet,
        BuilderState::ChannelUrlsPath: compressed_serialized_filesystem_resources_repository_builder::IsSet,
    {
        let video_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.get_video_urls_path())
            .await?;

        let playlist_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.get_playlist_urls_path())
            .await?;

        let channel_urls_file = ::tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.get_channel_urls_path())
            .await?;

        let mut output = self._build();

        output.video_urls_file.write(::tokio::sync::Mutex::new(video_urls_file));
        output.playlist_urls_file.write(::tokio::sync::Mutex::new(playlist_urls_file));
        output.channel_urls_file.write(::tokio::sync::Mutex::new(channel_urls_file));

        Ok(output)
    }
}

#[async_trait]
impl<State> UrlRepository for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default + ::core::marker::Send,
{
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
impl<State> Insert<VideoUrl> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default + ::core::marker::Send,
{
    async fn insert(self: ::std::sync::Arc<Self>, url: VideoUrl) -> Fallible<()> {
        use ::tokio::io::AsyncSeekExt as _;
        use ::tokio::io::AsyncWriteExt as _;

        let mut urls: ::std::collections::HashSet<VideoUrl, State> = ::std::sync::Arc::clone(&self).get().await?;
        urls.insert(url);

        let urls = urls.into_iter().map(Into::into).collect();

        let buffer = ::std::sync::Arc::clone(&self.serializer).serialize(urls)?;
        let buffer = ::std::sync::Arc::clone(&self.compressor).compress(buffer)?;

        let mut file = unsafe { self.video_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;
        file.set_len(0).await?;

        file.write_all(&buffer).await?;
        file.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl<State> Insert<PlaylistUrl> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default + ::core::marker::Send,
{
    async fn insert(self: ::std::sync::Arc<Self>, url: PlaylistUrl) -> Fallible<()> {
        use ::tokio::io::AsyncSeekExt as _;
        use ::tokio::io::AsyncWriteExt as _;

        let mut urls: ::std::collections::HashSet<PlaylistUrl, State> = ::std::sync::Arc::clone(&self).get().await?;
        urls.insert(url);

        let urls = urls.into_iter().map(Into::into).collect();

        let buffer = ::std::sync::Arc::clone(&self.serializer).serialize(urls)?;
        let buffer = ::std::sync::Arc::clone(&self.compressor).compress(buffer)?;

        let mut file = unsafe { self.playlist_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;
        file.set_len(0).await?;

        file.write_all(&buffer).await?;
        file.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl<State> Insert<ChannelUrl> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default + ::core::marker::Send,
{
    async fn insert(self: ::std::sync::Arc<Self>, url: ChannelUrl) -> Fallible<()> {
        use ::tokio::io::AsyncSeekExt as _;
        use ::tokio::io::AsyncWriteExt as _;

        let mut urls: ::std::collections::HashSet<ChannelUrl, State> = ::std::sync::Arc::clone(&self).get().await?;
        urls.insert(url);

        let urls = urls.into_iter().map(Into::into).collect();

        let buffer = ::std::sync::Arc::clone(&self.serializer).serialize(urls)?;
        let buffer = ::std::sync::Arc::clone(&self.compressor).compress(buffer)?;

        let mut file = unsafe { self.channel_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;
        file.set_len(0).await?;

        file.write_all(&buffer).await?;
        file.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl<State> Get<BoxedStream<VideoUrl>> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<VideoUrl>> {
        let urls: ::std::collections::HashSet<VideoUrl, State> = self.get().await?;
        Ok(::std::boxed::Box::pin(::futures::stream::iter(urls)))
    }
}

#[async_trait]
impl<State> Get<BoxedStream<PlaylistUrl>> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<PlaylistUrl>> {
        let urls: ::std::collections::HashSet<PlaylistUrl, State> = self.get().await?;
        Ok(::std::boxed::Box::pin(::futures::stream::iter(urls)))
    }
}

#[async_trait]
impl<State> Get<BoxedStream<ChannelUrl>> for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<BoxedStream<ChannelUrl>> {
        let urls: ::std::collections::HashSet<ChannelUrl, State> = self.get().await?;
        Ok(::std::boxed::Box::pin(::futures::stream::iter(urls)))
    }
}

#[async_trait]
impl<State> Get<::std::collections::HashSet<VideoUrl, State>>
    for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<::std::collections::HashSet<VideoUrl, State>> {
        use ::tokio::io::AsyncReadExt as _;
        use ::tokio::io::AsyncSeekExt as _;

        let mut file = unsafe { self.video_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let buffer = ::std::sync::Arc::clone(&self.compressor).decompress(buffer)?;
        let urls = ::std::sync::Arc::clone(&self.serializer).deserialize(buffer)?;

        let urls = urls.into_iter().map(Into::into).collect();

        Ok(urls)
    }
}

#[async_trait]
impl<State> Get<::std::collections::HashSet<PlaylistUrl, State>>
    for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<::std::collections::HashSet<PlaylistUrl, State>> {
        use ::tokio::io::AsyncReadExt as _;
        use ::tokio::io::AsyncSeekExt as _;

        let mut file = unsafe { self.playlist_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let buffer = ::std::sync::Arc::clone(&self.compressor).decompress(buffer)?;
        let urls = ::std::sync::Arc::clone(&self.serializer).deserialize(buffer)?;

        let urls = urls.into_iter().map(Into::into).collect();

        Ok(urls)
    }
}

#[async_trait]
impl<State> Get<::std::collections::HashSet<ChannelUrl, State>>
    for CompressedSerializedFilesystemResourcesRepository<State>
where
    State: ::std::hash::BuildHasher + Default,
{
    async fn get(self: ::std::sync::Arc<Self>) -> Fallible<::std::collections::HashSet<ChannelUrl, State>> {
        use ::tokio::io::AsyncReadExt as _;
        use ::tokio::io::AsyncSeekExt as _;

        let mut file = unsafe { self.channel_urls_file.assume_init_ref() }.lock().await;
        file.seek(::std::io::SeekFrom::Start(0)).await?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let buffer = ::std::sync::Arc::clone(&self.compressor).decompress(buffer)?;
        let urls = ::std::sync::Arc::clone(&self.serializer).deserialize(buffer)?;

        let urls = urls.into_iter().map(Into::into).collect();

        Ok(urls)
    }
}

type Buffer = Vec<u8>;

pub trait Serializer<Payload>: ::core::marker::Send + ::core::marker::Sync {
    fn serialize(self: ::std::sync::Arc<Self>, payload: Payload) -> Fallible<Buffer>;
    fn deserialize(self: ::std::sync::Arc<Self>, buffer: Buffer) -> Fallible<Payload>;
}

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct BincodeSerializer {
    configurations: ::bincode::config::Configuration,
}

impl<State> Serializer<::std::collections::HashSet<MaybeOwnedString, State>> for BincodeSerializer
where
    State: ::std::hash::BuildHasher + Default,
{
    fn serialize(
        self: ::std::sync::Arc<Self>, payload: ::std::collections::HashSet<MaybeOwnedString, State>,
    ) -> Fallible<Buffer> {
        let buffer = ::bincode::encode_to_vec(payload, self.configurations)?;

        Ok(buffer)
    }

    fn deserialize(
        self: ::std::sync::Arc<Self>, buffer: Buffer,
    ) -> Fallible<::std::collections::HashSet<MaybeOwnedString, State>> {
        if buffer.is_empty() {
            return Ok(Default::default());
        }

        let (payload, _) = ::bincode::decode_from_slice(&buffer, self.configurations)?;

        Ok(payload)
    }
}

pub trait Compressor: ::core::marker::Send + ::core::marker::Sync {
    fn compress(self: ::std::sync::Arc<Self>, buffer: Buffer) -> Fallible<Buffer>;
    fn decompress(self: ::std::sync::Arc<Self>, buffer: Buffer) -> Fallible<Buffer>;
}

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct Flate2Compressor {
    level: ::flate2::Compression,
}

impl Compressor for Flate2Compressor {
    fn compress(self: ::std::sync::Arc<Self>, buffer: Buffer) -> Fallible<Buffer> {
        use ::std::io::Write as _;

        let mut compressor = ::flate2::write::ZlibEncoder::new(Vec::new(), self.level);
        compressor.write_all(&buffer)?;

        Ok(compressor.finish()?)
    }

    fn decompress(self: ::std::sync::Arc<Self>, buffer: Buffer) -> Fallible<Buffer> {
        use ::std::io::Read as _;

        let mut decompressor = ::flate2::read::ZlibDecoder::new(&buffer[..]);

        let mut buffer = Vec::new();
        decompressor.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}
