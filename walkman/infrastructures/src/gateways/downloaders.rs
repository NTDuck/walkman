use ::async_trait::async_trait;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;
use ::use_cases::models::descriptors::PlaylistMetadata;
use ::use_cases::models::descriptors::UnresolvedVideo;
use ::use_cases::models::descriptors::VideoMetadata;
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
}

#[async_trait]
impl VideoDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: VideoUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        #[rustfmt::skip]
        let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
            &*url,
            "--paths", self.directory.to_str().ok()?,
            "--format", "bestaudio",
            "--extract-audio",
            "--audio-format", "mp3",
            "--output", "%(title)s.%(ext)s",
            "--quiet",
            "--newline",
            "--abort-on-error",
            "--no-playlist",
            "--color", "no_color",
            "--force-overwrites",
            "--progress",
            "--print", "before_dl:[video-started]%(id)s;%(webpage_url)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            "--progress-template", "[video-downloading]%(info.id)s;%(progress.eta)s;%(progress.elapsed)s;%(progress.downloaded_bytes)s;%(progress.total_bytes)s;%(progress.speed)s",
            "--print", "after_move:[video-completed]%(id)s;%(webpage_url)s;%(title)s;%(album)s;%(artist)s;%(genre)s;%(filepath)s",
        ])?;

        ::tokio::spawn({
            async move {
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
                )
            }
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

        #[rustfmt::skip]
        let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
            &*url,
            "--paths", self.directory.to_str().ok()?,
            "--quiet",
            "--flat-playlist",
            "--color", "no_color",
            "--print", "playlist:[playlist-started:metadata]%(id)s;%(webpage_url)s;%(title)s",
            "--print", "video:[playlist-started:video]%(id)s;(url)s"
        ])?;

        let playlist = ::tokio::spawn({
            let playlist_download_events_tx = playlist_download_events_tx.clone();
            let diagnostic_events_tx = diagnostic_events_tx.clone();

            async move {
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

                Ok::<_, ::anyhow::Error>(playlist)
            }
        })
        .await??;

        let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::new(0));
        let total = playlist.videos.as_deref().map(|videos| videos.len() as u64).unwrap_or_default();

        let resolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total as usize)));

        let playlist_id = playlist.id.clone();

        let unresolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(
                playlist
                    .videos
                    .as_deref()
                    .map(|videos| videos.iter().cloned().collect())
                    .unwrap_or_default(),
            ));

        let queue_emptied_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());

        (0..self.workers).for_each(|_| {
            ::tokio::spawn({
                let this = ::std::sync::Arc::clone(&self);

                let playlist_download_events_tx = playlist_download_events_tx.clone();
                let video_download_events_tx = video_download_events_tx.clone();
                let diagnostic_events_tx = diagnostic_events_tx.clone();

                let playlist_id = playlist_id.clone();
                let completed = ::std::sync::Arc::clone(&completed);
                let resolved_videos = ::std::sync::Arc::clone(&resolved_videos);
                let unresolved_videos = ::std::sync::Arc::clone(&unresolved_videos);
                let queue_emptied_notify = ::std::sync::Arc::clone(&queue_emptied_notify);

                async move {
                    loop {
                        let (video, queue_emptied_by_this_worker) = {
                            let mut unresolved_videos = unresolved_videos.lock().await;

                            let video = unresolved_videos.pop_front();
                            let queue_emptied_by_this_worker = unresolved_videos.is_empty();

                            (video, queue_emptied_by_this_worker)
                        };

                        let Some(video) = video else {
                            break;
                        };

                        let (video_download_events, diagnostic_events) =
                            VideoDownloader::download(::std::sync::Arc::clone(&this), video.url.into()).await?;

                        ::tokio::try_join!(
                            async {
                                video_download_events
                                    .map(Ok)
                                    .try_for_each(|event| async {
                                        if let VideoDownloadEvent::Completed(ref event) = event {
                                            completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                            resolved_videos.lock().await.push(event.video.clone());

                                            let event = PlaylistDownloadProgressUpdatedEvent {
                                                playlist_id: playlist_id.clone(),
                                                completed_videos: completed
                                                    .load(::std::sync::atomic::Ordering::Relaxed),
                                                total_videos: total,
                                            };

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

                        if queue_emptied_by_this_worker {
                            queue_emptied_notify.notify_one();
                        }

                        ::tokio::time::sleep(this.per_worker_cooldown).await;
                    }

                    Ok::<_, ::anyhow::Error>(())
                }
            });
        });

        ::tokio::spawn({
            let playlist_download_events_tx = playlist_download_events_tx.clone();

            let queue_emptied_notify = ::std::sync::Arc::clone(&queue_emptied_notify);

            async move {
                queue_emptied_notify.notified().await;

                let videos = ::std::mem::take(&mut *resolved_videos.lock().await);
                let videos = videos.is_empty().not().then_some(videos.into());

                let playlist = ResolvedPlaylist {
                    url: playlist.url,
                    id: playlist.id,
                    metadata: playlist.metadata,
                    videos,
                };

                let event = PlaylistDownloadCompletedEvent { playlist };
                playlist_download_events_tx.send(PlaylistDownloadEvent::Completed(event))?;

                Ok::<_, ::anyhow::Error>(())
            }
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

trait CommandExecutor {
    type Stdout: ::futures::Stream<Item = MaybeOwnedString>;
    type Stderr: ::futures::Stream<Item = MaybeOwnedString>;

    fn execute<Program, Args>(program: Program, args: Args) -> Fallible<(Self::Stdout, Self::Stderr)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>;
}

struct TokioCommandExecutor;

impl CommandExecutor for TokioCommandExecutor {
    type Stderr = ::tokio_stream::wrappers::UnboundedReceiverStream<MaybeOwnedString>;
    type Stdout = ::tokio_stream::wrappers::UnboundedReceiverStream<MaybeOwnedString>;

    fn execute<Program, Args>(program: Program, args: Args) -> Fallible<(Self::Stdout, Self::Stderr)>
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
            ::tokio_stream::wrappers::UnboundedReceiverStream::new(stdout_rx),
            ::tokio_stream::wrappers::UnboundedReceiverStream::new(stderr_rx),
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
                        .title(title.singlevalued()?)
                        .album(album.singlevalued()?)
                        .artists(artists.multivalued()?)
                        .genres(genres.multivalued()?)
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
                        .title(title.singlevalued()?)
                        .album(album.singlevalued()?)
                        .artists(artists.multivalued()?)
                        .genres(genres.multivalued()?)
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
        let attrs = line.as_ref().split(':');
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
        use ::futures::StreamExt as _;

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
                let [id, title, url] = YtdlpAttributes::parse(attrs)?.into();

                let playlist = PartiallyResolvedPlaylist::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .metadata(PlaylistMetadata::builder()
                        .maybe_title(title.singlevalued())
                        .build())
                    .maybe_videos(videos.is_empty().not().then(|| videos.into()))
                    .build();

                return Some(Self { playlist });
            }
        }

        None
    }
}

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
