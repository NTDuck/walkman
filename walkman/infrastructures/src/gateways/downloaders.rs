
use std::ops::Not;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::PlaylistMetadata;
use ::domain::VideoMetadata;
use ::use_cases::gateways::PlaylistDownloader;
use ::use_cases::gateways::VideoDownloader;
use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
use ::use_cases::models::descriptors::PartiallyResolvedVideo;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::use_cases::models::descriptors::UnresolvedPlaylist;
use ::use_cases::models::descriptors::UnresolvedVideo;
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

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;
use crate::utils::extensions::OptionExt;

#[derive(new)]
pub struct YtdlpDownloader {
    configurations: YtdlpConfigurations,
}

pub struct YtdlpConfigurations {
    pub directory: MaybeOwnedPath,
    pub workers: u64,
    pub cooldown: ::std::time::Duration,
}

#[async_trait]
impl VideoDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, video: UnresolvedVideo,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        use ::futures::StreamExt as _;
        use ::futures::TryStreamExt as _;

        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        #[rustfmt::skip]
        let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
            &*video.url,
            "--paths", &self.configurations.directory.to_str().ok()?,
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
            "--print", "before_dl:[video-started]%(webpage_url)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            "--progress-template", "[video-downloading]%(info.id)s;%(progress.eta)s;%(progress.elapsed)s;%(progress.downloaded_bytes)s;%(progress.total_bytes)s;%(progress.speed)s",
            "--print", "after_move:[video-completed]%(webpage_url)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s;%(filepath)s",
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

// #[async_trait]
// impl<CommandExecutorImpl> PlaylistDownloader for YtdlpDownloader<CommandExecutorImpl>
// where
//     CommandExecutorImpl: CommandExecutor'static,
// {
//     async fn download(
//         self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist,
//     ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
//         use ::std::ops::Not as _;
//         use ::futures::StreamExt as _;
//         use ::futures::TryStreamExt as _;

//         let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
//         let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
//         let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

//         let (stdout, stderr) = ::std::sync::Arc::clone(&self.command_executor).execute("yt-dlp", [
//             &*playlist.url,
//             "--paths", self.configurations.directory.to_str().some()?,
//             "--quiet",
//             "--flat-playlist",
//             "--color", "no_color",
//             "--print", "playlist:[playlist-started:metadata]%(id)s;%(title)s;%(webpage_url)s",
//             "--print", "video:[playlist-started:url]%(url)s"
//         ])?;

//         let correlation_id = ::std::sync::Arc::clone(&self.id_generator).generate();

//         let playlist = ::tokio::spawn({
//             let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();
//             let correlation_id = correlation_id.clone();

//             let playlist_download_events_tx = playlist_download_events_tx.clone();
//             let diagnostic_events_tx = diagnostic_events_tx.clone();

//             async move {
//                 let (playlist, _) = ::tokio::try_join!(
//                     async {
//                         let payload = PlaylistDownloadStartedEvent::from_lines(stdout).await.some()?;
//                         let playlist = payload.playlist.clone();

//                         let event = Event {
//                             metadata: EventMetadata {
//                                 worker_id: worker_id.clone(),
//                                 correlation_id: correlation_id.clone(),
//                                 timestamp: ::std::time::SystemTime::now(),
//                             },
//                             payload: PlaylistDownloadEvent::Started(payload),
//                         };

//                         playlist_download_events_tx.send(event)?;
//                         Ok(playlist)
//                     },

//                     async {
//                         stderr
//                             .filter_map(|line| async { DiagnosticEvent::from_line(line) })
//                             .map(|payload| Event {
//                                 metadata: EventMetadata {
//                                     worker_id: worker_id.clone(),
//                                     correlation_id: correlation_id.clone(),
//                                     timestamp: ::std::time::SystemTime::now(),
//                                 },
//                                 payload,
//                             })
//                             .map(Ok)
//                             .try_for_each(|event| async { diagnostic_events_tx.send(event) })
//                             .await
//                             .map_err(::anyhow::Error::from)
//                     },
//                 )?;

//                 Ok::<_, ::anyhow::Error>(playlist)
//             }
//         })
//         .await??;

//         let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicUsize::new(0));
//         let total = playlist.videos
//             .as_deref()
//             .map_or(0, |videos| videos.len());

//         let resolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
//             ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total)));

//         let unresolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
//             ::std::sync::Arc::new(::tokio::sync::Mutex::new(playlist.videos.as_deref().map_or_else(|| ::std::collections::VecDeque::new(), |videos| videos.iter().cloned().collect())));
        
//         let unresolved_videos_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());
        
//         (0..self.configurations.workers).for_each(|_| {
//             ::tokio::spawn({
//                 let this = ::std::sync::Arc::clone(&self);

//                 let playlist_download_events_tx = playlist_download_events_tx.clone();
//                 let video_download_events_tx = video_download_events_tx.clone();
//                 let diagnostic_events_tx = diagnostic_events_tx.clone();

//                 let worker_id = ::std::sync::Arc::clone(&this.id_generator).generate();
//                 let correlation_id = correlation_id.clone();

//                 let directory = directory.clone();
//                 let completed = ::std::sync::Arc::clone(&completed);
//                 let resolved_videos = ::std::sync::Arc::clone(&resolved_videos);
//                 let unresolved_videos = ::std::sync::Arc::clone(&unresolved_videos);
//                 let unresolved_videos_notify = ::std::sync::Arc::clone(&unresolved_videos_notify);

//                 async move {
//                     loop {
//                         let (video, is_last_worker_to_poll) = {
//                             let mut unresolved_videos = unresolved_videos.lock().await;
//                             let video = unresolved_videos.pop_front();
//                             let is_last_worker_to_poll = unresolved_videos.is_empty();
//                             (video, is_last_worker_to_poll)
//                         };

//                         let Some(video) = video else { break };

//                         let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&this).download_video(video.url.clone(), directory.clone()).await?;

//                         ::tokio::try_join!(
//                             async {
//                                 video_download_events
//                                     .map(|event| event.with_metadata(EventMetadata {
//                                         worker_id: worker_id.clone(),
//                                         correlation_id: correlation_id.clone(),
//                                         timestamp: ::std::time::SystemTime::now(),
//                                     }))
//                                     .map(Ok)
//                                     .try_for_each(|event| async {
//                                         match event.payload {
//                                             VideoDownloadEvent::Completed(ref payload) => {
//                                                 completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
//                                                 resolved_videos.lock().await.push(payload.video.clone());

//                                                 let event = Event {
//                                                     metadata: EventMetadata {
//                                                         worker_id: worker_id.clone(),
//                                                         correlation_id: correlation_id.clone(),
//                                                         timestamp: ::std::time::SystemTime::now(),
//                                                     },
//                                                     payload: PlaylistDownloadEvent::ProgressUpdated(
//                                                         PlaylistDownloadProgressUpdatedEvent {
//                                                             video: payload.video.clone(),
//                                                             completed_videos: completed.load(::std::sync::atomic::Ordering::Relaxed),
//                                                             total_videos,
//                                                         },
//                                                     ),
//                                                 };

//                                                 playlist_download_events_tx.send(event)?;
//                                             },
//                                             _ => {},
//                                         }

//                                         video_download_events_tx.send(event)?;
//                                         Ok(())
//                                     })
//                                     .await
//                             },

//                             async {
//                                 diagnostic_events
//                                     .map(|event| Event {
//                                         metadata: EventMetadata {
//                                             worker_id: worker_id.clone(),
//                                             correlation_id: correlation_id.clone(),
//                                             timestamp: ::std::time::SystemTime::now(),
//                                         },
//                                         payload: event.payload,
//                                     })
//                                     .map(Ok)
//                                     .try_for_each(|event| async { diagnostic_events_tx.send(event) })
//                                     .await
//                                     .map_err(::anyhow::Error::from)
//                             },
//                         )?;
                        
//                         if is_last_worker_to_poll {
//                             unresolved_videos_notify.notify_one();
//                         }

//                         ::tokio::time::sleep(this.configurations.cooldown).await;
//                     }

//                     Ok::<_, ::anyhow::Error>(())
//                 }
//             });
//         });

//         ::tokio::spawn({
//             let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();
//             let correlation_id = correlation_id.clone();

//             let playlist_download_events_tx = playlist_download_events_tx.clone();

//             let unresolved_videos_notify = ::std::sync::Arc::clone(&unresolved_videos_notify);

//             async move {
//                 unresolved_videos_notify.notified().await;

//                 let playlist = ResolvedPlaylist {
//                     url: playlist.url,
//                     id: playlist.id,
//                     metadata: PlaylistMetadata {
//                         title: playlist.metadata.title,
//                     },
//                     videos: {
//                         let videos = ::std::mem::take(&mut *resolved_videos.lock().await);
//                         videos.is_empty().not().then_some(videos)
//                     }
//                 };

//                 let event = Event {
//                     metadata: EventMetadata {
//                         worker_id,
//                         correlation_id,
//                         timestamp: ::std::time::SystemTime::now(),
//                     },
//                     payload: PlaylistDownloadEvent::Completed(PlaylistDownloadCompletedEvent { playlist }),
//                 };

//                 playlist_download_events_tx.send(event)?;

//                 Ok::<_, ::anyhow::Error>(())
//             }
//         });

//         Ok((
//             ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
//             ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
//             ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
//         ))
//     }
// }

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
    type Stdout = ::tokio_stream::wrappers::UnboundedReceiverStream<MaybeOwnedString>;
    type Stderr = ::tokio_stream::wrappers::UnboundedReceiverStream<MaybeOwnedString>;

    fn execute<Program, Args>(program: Program, args: Args) -> Fallible<(Self::Stdout, Self::Stderr)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>,
    {
        use ::tokio::io::AsyncBufReadExt as _;
        use ::futures::StreamExt as _;
        use ::futures::TryStreamExt as _;

        let (stdout_tx, stdout_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let mut process = ::tokio::process::Command::new(program)
            .args(args)
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().ok()?;
        let stderr = process.stderr.take().ok()?;

        ::tokio::task::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stdout).lines();
            
            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| line.to_owned().into())
                .map(Ok)
                .try_for_each(|line| async { stdout_tx.send(line) })
                .await
        });

        ::tokio::task::spawn(async move {
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

trait FromLine {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized;
}

impl FromLine for VideoDownloadEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let line = line.as_ref();

        VideoDownloadProgressUpdatedEvent::from_line(line).map(Self::ProgressUpdated)
            .or(VideoDownloadStartedEvent::from_line(line).map(Self::Started))
            .or(VideoDownloadCompletedEvent::from_line(line).map(Self::Completed))
    }
}

impl FromLine for VideoDownloadStartedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let mut attrs = line.as_ref().strip_prefix("[video-started]")?.split(';');

        let url = parse_attr(attrs.next()?)?;
        let id = parse_attr(attrs.next()?)?;
        let title = parse_attr(attrs.next()?);
        let album = parse_attr(attrs.next()?);
        let artists = parse_multivalued_attr(attrs.next()?);
        let genres = parse_multivalued_attr(attrs.next()?);

        let video = PartiallyResolvedVideo {
            url,
            id,
            metadata: VideoMetadata {
                title,
                album,
                artists,
                genres,
            },
        };

        Some(Self { video })
    }
}

impl FromLine for VideoDownloadProgressUpdatedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let mut attrs = line.as_ref().strip_prefix("[video-downloading]")?.split(';');

        let id = parse_attr(attrs.next()?)?;
        let eta = parse_attr(attrs.next()?)?;
        let elapsed = parse_attr(attrs.next()?)?;
        let downloaded_bytes = parse_attr(attrs.next()?)?;
        let total_bytes = parse_attr(attrs.next()?)?;
        let bytes_per_second = parse_attr(attrs.next()?)?;

        let eta = ::std::time::Duration::from_secs(eta.parse().ok()?);
        let elapsed = ::std::time::Duration::try_from_secs_f64(elapsed.parse().ok()?).ok()?;
        let downloaded_bytes = downloaded_bytes.parse().ok()?;
        let total_bytes = total_bytes.parse().ok()?;
        let bytes_per_second = bytes_per_second.parse().ok()?;

        Some(Self {
            id,
            eta,
            elapsed,
            downloaded_bytes,
            total_bytes,
            bytes_per_second,
        })
    }
}

impl FromLine for VideoDownloadCompletedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let mut attrs = line.as_ref().strip_prefix("[video-completed]")?.split(';');

        let url = parse_attr(attrs.next()?)?;
        let id = parse_attr(attrs.next()?)?;
        let title = parse_attr(attrs.next()?);
        let album = parse_attr(attrs.next()?);
        let artists = parse_multivalued_attr(attrs.next()?);
        let genres = parse_multivalued_attr(attrs.next()?);
        let path = parse_attr(attrs.next()?)?;

        let path = match path {
            MaybeOwnedString::Borrowed(path) => MaybeOwnedPath::Borrowed(path.as_ref()),
            MaybeOwnedString::Owned(path) => MaybeOwnedPath::Owned(path.into()),
        };

        let video = ResolvedVideo {
            url,
            id,
            metadata: VideoMetadata { title, album, artists, genres },
            path,
        };

        Some(Self { video })
    }
}

impl FromLine for DiagnosticEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let mut attrs = line.as_ref().split(':');

        let level = parse_attr(attrs.next()?)?;
        let message = parse_attr(attrs.next()?)?;

        let level = match level.as_ref() {
            "WARNING" => DiagnosticLevel::Warning,
            "ERROR" => DiagnosticLevel::Error,
            _ => return None,
        };

        Some(Self { level, message })
    }    
}

#[async_trait]
trait FromLines: Send + Sync {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized;
}

#[async_trait]
impl FromLines for PlaylistDownloadStartedEvent {
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
            if let Some(line) = line.as_ref().strip_prefix("[playlist-started:url]") {
                let mut attrs = line.split(';');

                let url = parse_attr(attrs.next()?)?;

                let video = UnresolvedVideo { url };
                videos.push(video);

            } else if let Some(line) = line.as_ref().strip_prefix("[playlist-started:metadata]") {
                let mut attrs = line.split(';');

                let id = parse_attr(attrs.next()?)?;
                let title = parse_attr(attrs.next()?);
                let url = parse_attr(attrs.next()?)?;

                let videos = videos.is_empty().not().then(|| videos.into());

                let playlist = PartiallyResolvedPlaylist {
                    url,
                    id,
                    metadata: PlaylistMetadata { title },
                    videos,
                };

                return Some(Self { playlist })
            }
        }

        None
    }
}

/// TODO: Try making borrowing works
fn parse_multivalued_attr(string: &str) -> Option<MaybeOwnedVec<MaybeOwnedString>> {
    let attr = parse_attr(string)?;

    let attrs = attr
        .split(',')
        .map(parse_attr)
        .flatten()
        .collect::<Vec<_>>();

    Some(attrs.into())
}

fn parse_attr(string: &str) -> Option<MaybeOwnedString> {
    let string = normalize(string);

    if string == "NA" {
        None
    } else {
        Some(string.to_owned().into())
    }
}

fn normalize(string: &str) -> &str {
    string.trim()
}
