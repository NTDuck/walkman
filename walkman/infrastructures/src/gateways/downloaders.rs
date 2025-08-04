use ::async_trait::async_trait;
use ::domain::ChannelUrl;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;
use ::use_cases::gateways::ChannelDownloader;
use use_cases::models::descriptors::ChannelMetadata;
use use_cases::models::descriptors::PartiallyResolvedChannel;
use ::use_cases::models::descriptors::PlaylistMetadata;
use use_cases::models::descriptors::UnresolvedPlaylist;
use ::use_cases::models::descriptors::UnresolvedVideo;
use ::use_cases::models::descriptors::VideoMetadata;
use ::use_cases::models::events::ChannelDownloadEvent;
use use_cases::models::events::ChannelDownloadStartedEvent;
use ::std::ops::Not;
use ::use_cases::gateways::PlaylistDownloader;
use ::use_cases::gateways::VideoDownloader;
use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
use ::use_cases::models::descriptors::PartiallyResolvedVideo;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::use_cases::models::events::DiagnosticEvent;
use ::use_cases::models::events::DiagnosticLevel;
use ::use_cases::models::events::PlaylistDownloadCompletedEvent;
use ::use_cases::models::events::PlaylistDownloadEvent;
use ::use_cases::models::events::PlaylistDownloadProgressUpdatedEvent;
use ::use_cases::models::events::PlaylistDownloadStartedEvent;
use ::use_cases::models::events::VideoDownloadCompletedEvent;
use ::use_cases::models::events::VideoDownloadEvent;
use ::use_cases::models::events::VideoDownloadProgressUpdatedEvent;
use ::use_cases::models::events::VideoDownloadStartedEvent;
use ::futures::prelude::*;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;
use crate::utils::extensions::OptionExt;

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct YtdlpDownloader {
    directory: MaybeOwnedPath,
    workers: u64,
    per_worker_cooldown: ::std::time::Duration,

    #[builder(skip = ::std::sync::Arc::new(::tokio::sync::Semaphore::new(workers as usize)))]
    worker_pool: ::std::sync::Arc<::tokio::sync::Semaphore>,
}

#[async_trait]
impl VideoDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: VideoUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        ::tokio::spawn(async move {
            #[rustfmt::skip]
            let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
                &*url,
                "--quiet",
                "--color", "no_color",
                "--paths", self.directory.to_str().ok()?,
                "--no-playlist",
                "--format", "bestaudio",
                "--extract-audio",
                "--audio-format", "mp3",
                "--output", "%(title)s.%(ext)s",
                "--newline",
                "--abort-on-error",
                "--force-overwrites",
                "--progress",
                "--print", "before_dl:[video-started]%(id)s;%(webpage_url)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
                "--progress-template", "[video-downloading]%(info.id)s;%(progress.eta)s;%(progress.elapsed)s;%(progress.downloaded_bytes)s;%(progress.total_bytes)s;%(progress.speed)s",
                "--print", "after_move:[video-completed]%(id)s;%(webpage_url)s;%(title)s;%(album)s;%(artist)s;%(genre)s;%(filepath)s",
            ])?;

            ::tokio::try_join!(
                async {
                    stdout
                        .filter_map(|line| async { VideoDownloadEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { video_download_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
                async {
                    stderr
                        .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
            )?;

            Ok::<_, ::anyhow::Error>(())
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

#[async_trait]
impl PlaylistDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: PlaylistUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<DiagnosticEvent>)>
    {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        ::tokio::spawn(async move {
            #[rustfmt::skip]
            let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
                &*url,
                "--quiet",
                "--color", "no_color",
                "--flat-playlist",
                "--yes-playlist",
                "--print", "playlist:[playlist-started:metadata]%(id)s;%(webpage_url)s;%(title)s",
                "--print", "video:[playlist-started:video]%(id)s;%(url)s"
            ])?;

            let (playlist, _) = ::tokio::try_join!(
                async {
                    let event = PlaylistDownloadStartedEvent::from_lines(stdout).await.ok()?;
                    let playlist = event.playlist.clone();

                    playlist_download_events_tx.send(PlaylistDownloadEvent::Started(event))?;

                    Ok(playlist)
                },
                async {
                    stderr
                        .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
            )?;

            let completed_videos = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::new(0));
            let total_videos = playlist.videos.as_deref().map(|videos| videos.len() as u64).unwrap_or_default();

            let videos = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_videos as usize)));

            let videos_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());

            playlist.videos
                .as_deref()
                .into_iter()
                .flatten()
                .cloned()
                .for_each(|video| {
                    ::tokio::spawn({
                        let this = ::std::sync::Arc::clone(&self);

                        let video_download_events_tx = video_download_events_tx.clone();
                        let playlist_download_events_tx = playlist_download_events_tx.clone();
                        let diagnostic_events_tx = diagnostic_events_tx.clone();

                        let playlist_id = playlist.id.clone();

                        let completed_videos = ::std::sync::Arc::clone(&completed_videos);
                        let videos = ::std::sync::Arc::clone(&videos);
                        let videos_completed_notify = ::std::sync::Arc::clone(&videos_completed_notify);

                        async move {
                            let worker = this.worker_pool.acquire().await?;

                            let (video_download_events, diagnostic_events) = VideoDownloader::download(::std::sync::Arc::clone(&this), video.url.clone().into()).await?;

                            ::tokio::try_join!(
                                async {
                                    video_download_events
                                        .map(Ok)
                                        .try_for_each(|event| async {
                                            if let VideoDownloadEvent::Completed(ref event) = event {
                                                completed_videos.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                                videos.lock().await.push(event.video.clone());

                                                let event = PlaylistDownloadProgressUpdatedEvent::builder()
                                                    .playlist_id(playlist_id.clone())
                                                    .completed_videos(completed_videos.load(::std::sync::atomic::Ordering::Relaxed))
                                                    .total_videos(total_videos)
                                                    .build();

                                                playlist_download_events_tx
                                                    .send(PlaylistDownloadEvent::ProgressUpdated(event))?;
                                            }

                                            video_download_events_tx.send(event)?;

                                            Ok::<_, ::anyhow::Error>(())
                                        })
                                        .await
                                },
                                async {
                                    diagnostic_events
                                        .map(Ok)
                                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                                        .await
                                        .map_err(::anyhow::Error::from)
                                },
                            )?;

                            ::tokio::time::sleep(this.per_worker_cooldown).await;
                            ::core::mem::drop(worker);

                            if completed_videos.load(::std::sync::atomic::Ordering::Relaxed) == total_videos {
                                videos_completed_notify.notify_one();
                            }

                            Fallible::Ok(())
                        }
                    });
                });

            videos_completed_notify.notified().await;

            let videos = ::std::mem::take(&mut *videos.lock().await);
            let videos = videos.is_empty().not().then_some(videos.into());

            let playlist = ResolvedPlaylist::builder()
                .id(playlist.id)
                .url(playlist.url)
                .metadata(playlist.metadata)
                .videos(videos)
                .build();

            let event = PlaylistDownloadCompletedEvent { playlist };
            playlist_download_events_tx.send(PlaylistDownloadEvent::Completed(event))?;

            Ok::<_, ::anyhow::Error>(())
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

#[async_trait]
impl ChannelDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: ChannelUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<ChannelDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (channel_download_events_tx, channel_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        // TODO: Make this not ugly!
        #[rustfmt::skip]
        let (stdout, stderr) = TokioCommandExecutor::execute_all([
            ("yt-dlp", [
                &*url,
                "--quiet",
                "--color", "no_color",
                "--print", "[channel-started:metadata]%(channel_url)s;%(channel_id)s;%(channel)s",
                "",
            ]),
            ("yt-dlp", [
                &format!("{}/videos", &*url),
                "--quiet",
                "--color", "no_color",
                "--print", "[channel-started:video]%(id)s;%(webpage_url)s",
                "",
            ]),
            ("yt-dlp", [
                &format!("{}/playlists", &*url),
                "--quiet",
                "--color", "no_color",
                "--flat-playlist",
                "--print", "%(id)s;%(url)s",
            ]),
        ])?;

        let channel = ::tokio::spawn({
            let channel_download_events_tx = channel_download_events_tx.clone();
            let diagnostic_events_tx = diagnostic_events_tx.clone();

            async move {
                let (channel, _) = ::tokio::try_join!(
                    async {
                        let event = ChannelDownloadStartedEvent::from_lines(stdout).await.ok()?;
                        let channel = event.channel.clone();

                        channel_download_events_tx.send(ChannelDownloadEvent::Started(event))?;

                        Ok(channel)
                    },
                    async {
                        stderr
                            .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                            .map(Ok)
                            .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                            .await
                            .map_err(::anyhow::Error::from)
                    },
                )?;

                Ok::<_, ::anyhow::Error>(channel)
            }
        })
        .await??;

        // let channel_id = channel.id.clone();

        // let completed_videos = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::new(0));
        // let total_videos = channel.videos.as_deref().map(|videos| videos.len() as u64).unwrap_or_default();
        // let completed_playlists = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::new(0));
        // let total_playlists = channel.playlists.as_deref().map(|playlists| playlists.len() as u64).unwrap_or_default();

        // let resolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_videos as usize)));

        // let unresolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(
        //         channel
        //             .videos
        //             .as_deref()
        //             .map(|videos| videos.iter().cloned().collect())
        //             .unwrap_or_default(),
        //     ));

        // let resolved_playlists: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_playlists as usize)));

        // let unresolved_playlists: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(
        //         channel
        //             .playlists
        //             .as_deref()
        //             .map(|playlists| playlists.iter().cloned().collect())
        //             .unwrap_or_default(),
        //     ));

        // let videos_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());
        // let playlists_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(channel_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

trait CommandExecutor {
    fn execute<Program, Args>(program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>;

    fn execute_all<Program, Args, const N: usize>(commands: [(Program, Args); N]) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>,
    {
        let mut stdouts = Vec::with_capacity(N);
        let mut stderrs = Vec::with_capacity(N);

        commands
            .into_iter()
            .filter_map(|(program, args)| Self::execute(program, args).ok())
            .for_each(|(stdout, stderr)| {
                stdouts.push(stdout);
                stderrs.push(stderr);
            });

        Ok((
            ::std::boxed::Box::pin(::futures::stream::select_all(stdouts)),
            ::std::boxed::Box::pin(::futures::stream::select_all(stderrs)),
        ))
    }
}

struct TokioCommandExecutor;

impl CommandExecutor for TokioCommandExecutor {
    fn execute<Program, Args>(program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>,
    {
        use ::tokio::io::AsyncBufReadExt as _;

        let (stdout_tx, stdout_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let mut process = ::tokio::process::Command::new(program)
            .args(args)
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().ok()?;
        let stderr = process.stderr.take().ok()?;

        ::tokio::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stdout).lines();

            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| line.to_owned().into())
                .map(Ok)
                .try_for_each(|line| async { stdout_tx.send(line) })
                .await
        });

        ::tokio::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stderr).lines();

            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| line.to_owned().into())
                .map(Ok)
                .try_for_each(|line| async { stderr_tx.send(line) })
                .await
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stdout_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stderr_rx)),
        ))
    }
}

trait FromYtdlpLine: ::core::marker::Send + ::core::marker::Sync {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized;
}

impl FromYtdlpLine for VideoDownloadEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let line = line.as_ref();

        VideoDownloadProgressUpdatedEvent::from_line(line)
            .map(Self::ProgressUpdated)
            .or(VideoDownloadStartedEvent::from_line(line).map(Self::Started))
            .or(VideoDownloadCompletedEvent::from_line(line).map(Self::Completed))
    }
}

impl FromYtdlpLine for VideoDownloadStartedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-started]")?.split(';');
        let [id, url, title, album, artists, genres] = YtdlpAttributes::parse(attrs)?.into();

        Some(
            Self::builder()
                .video(PartiallyResolvedVideo::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .metadata(VideoMetadata::builder()
                        .title(title.singlevalued())
                        .album(album.singlevalued())
                        .artists(artists.multivalued())
                        .genres(genres.multivalued())
                        .build())
                    .build())
                .build()
        )
    }
}

impl FromYtdlpLine for VideoDownloadProgressUpdatedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-downloading]")?.split(';');
        let [id, eta, elapsed, downloaded_bytes, total_bytes, bytes_per_second] = YtdlpAttributes::parse(attrs)?.into();

        Some(
            Self::builder()
                .video_id(id.singlevalued()?)
                .eta(::std::time::Duration::from_secs(eta.singlevalued()?.parse().ok()?))
                .elapsed(::std::time::Duration::try_from_secs_f64(elapsed.singlevalued()?.parse().ok()?).ok()?)
                .downloaded_bytes(downloaded_bytes.singlevalued()?.parse().ok()?)
                .total_bytes(total_bytes.singlevalued()?.parse().ok()?)
                .bytes_per_second(bytes_per_second.singlevalued()?.parse::<f64>().ok()?.floor() as u64)
                .build()
        )
    }
}

impl FromYtdlpLine for VideoDownloadCompletedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-completed]")?.split(';');
        let [id, url, title, album, artists, genres, path] = YtdlpAttributes::parse(attrs)?.into();

        Some(
            Self::builder()
                .video(ResolvedVideo::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .metadata(VideoMetadata::builder()
                        .title(title.singlevalued())
                        .album(album.singlevalued())
                        .artists(artists.multivalued())
                        .genres(genres.multivalued())
                        .build())
                    .path(match path.singlevalued()? {
                        MaybeOwnedString::Borrowed(path) => MaybeOwnedPath::Borrowed(path.as_ref()),
                        MaybeOwnedString::Owned(path) => MaybeOwnedPath::Owned(path.into()),
                    })
                    .build())
                .build()
        )
    }
}

impl FromYtdlpLine for DiagnosticEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().splitn(2, ':');
        let [level, message] = YtdlpAttributes::parse(attrs)?.into();

        Some(
            Self::builder()
                .level(match level.singlevalued()?.as_ref() {
                    "WARNING" => DiagnosticLevel::Warning,
                    "ERROR" => DiagnosticLevel::Error,
                    _ => return None,
                })
                .message(message.singlevalued()?)
                .build()
        )
    }
}

#[async_trait]
trait FromYtdlpLines: ::core::marker::Send + ::core::marker::Sync {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized;
}

#[async_trait]
impl FromYtdlpLines for PlaylistDownloadStartedEvent {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized,
    {
        let (mut id, mut url, mut title) = (None, None, None);
        let mut videos = Vec::new();

        ::futures::pin_mut!(lines);

        while let Some(line) = lines.next().await {
            if let Some(line) = line.as_ref().strip_prefix("[playlist-started:video]") {
                let attrs = line.split(';');
                let [id, url] = YtdlpAttributes::parse(attrs)?.into();

                let video = UnresolvedVideo::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .build();

                videos.push(video);
                
            } else if let Some(line) = line.as_ref().strip_prefix("[playlist-started:metadata]") {
                let attrs = line.split(';');
                let [_id, _url, _title] = YtdlpAttributes::parse(attrs)?.into();

                id = _id.singlevalued();
                url = _url.singlevalued();
                title = _title.singlevalued();
            }
        }

        let playlist = PartiallyResolvedPlaylist::builder()
            .id(id?)
            .url(url?)
            .metadata(PlaylistMetadata::builder()
                .title(title)
                .build())
            .videos(videos.is_empty().not().then(|| videos.into()))
            .build();

        Some(Self { playlist })
    }
}

#[async_trait]
impl FromYtdlpLines for ChannelDownloadStartedEvent {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized,
    {
        let (mut id, mut url, mut title) = (None, None, None);
        let mut videos = Vec::new();
        let mut playlists = Vec::new();

        ::futures::pin_mut!(lines);

        while let Some(line) = lines.next().await {
            if let Some(line) = line.as_ref().strip_prefix("[channel-started:video]") {
                let attrs = line.split(';');
                let [id, url] = YtdlpAttributes::parse(attrs)?.into();

                let video = UnresolvedVideo::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .build();

                videos.push(video);

            } else if let Some(line) = line.as_ref().strip_prefix("[channel-started:playlist]") {
                let attrs = line.split(';');
                let [id, url] = YtdlpAttributes::parse(attrs)?.into();

                let playlist = UnresolvedPlaylist::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .build();

                playlists.push(playlist);

            } else if let Some(line) = line.as_ref().strip_prefix("[channel-started:metadata]") {
                let attrs = line.split(';');
                let [_id, _url, _title] = YtdlpAttributes::parse(attrs)?.into();

                id = _id.singlevalued();
                url = _url.singlevalued();
                title = _title.singlevalued();
            }
        }

        let channel = PartiallyResolvedChannel::builder()
            .id(id?)
            .url(url?)
            .metadata(ChannelMetadata::builder()
                .title(title)
                .build())
            .videos(videos.is_empty().not().then(|| videos.into()))
            .playlists(playlists.is_empty().not().then(|| playlists.into()))
            .build();

        Some(Self { channel })
    }
}

#[derive(Clone)]
struct YtdlpAttribute<'a>(&'a str);

impl<'a> YtdlpAttribute<'a> {
    fn singlevalued(self) -> Option<MaybeOwnedString> {
        match self.0.trim() {
            "NA" => None,
            attr => Some(attr.to_owned().into()),
        }
    }

    fn multivalued(self) -> Option<MaybeOwnedVec<MaybeOwnedString>> {
        let attrs = self.0
            .split(',')
            .map(YtdlpAttribute)
            .filter_map(Self::singlevalued)
            .collect::<Vec<_>>();

        Some(attrs.into())
    }
}

struct YtdlpAttributes<'a, const N: usize>([YtdlpAttribute<'a>; N]);

impl<'a, const N: usize> From<YtdlpAttributes<'a, N>> for [YtdlpAttribute<'a>; N] {
    fn from(outer: YtdlpAttributes<'a, N>) -> Self {
        outer.0
    }
}

impl<'a, const N: usize> YtdlpAttributes<'a, N> {
    fn parse<Attrs>(attrs: Attrs) -> Option<Self>
    where
        Attrs: Iterator<Item = &'a str>,
    {
        let attrs = attrs
            .map(YtdlpAttribute)
            .collect::<Vec<_>>()
            .try_into()
            .ok()?;

        Some(Self(attrs))
    }
}
