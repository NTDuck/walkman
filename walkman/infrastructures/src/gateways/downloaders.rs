
use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::PlaylistMetadata;
use ::domain::VideoMetadata;
use use_cases::gateways::PlaylistDownloader;
use use_cases::gateways::VideoDownloader;
use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
use ::use_cases::models::descriptors::PartiallyResolvedVideo;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use use_cases::models::descriptors::UnresolvedPlaylist;
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
        let (stdout, stderr) = ::std::sync::Arc::clone(&self.command_executor).execute("yt-dlp", [
            &*video.url,
            "--paths", &self.configurations.directory.to_str().some()?,
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
            "--progress-template", "[video-downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s",
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

#[async_trait]
impl<CommandExecutorImpl> PlaylistDownloader for YtdlpDownloader<CommandExecutorImpl>
where
    CommandExecutorImpl: CommandExecutor + 'static,
{
    async fn download(
        self: ::std::sync::Arc<Self>, playlist: UnresolvedPlaylist,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        use ::std::ops::Not as _;
        use ::futures::StreamExt as _;
        use ::futures::TryStreamExt as _;

        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let (stdout, stderr) = ::std::sync::Arc::clone(&self.command_executor).execute("yt-dlp", [
            &*playlist.url,
            "--paths", self.configurations.directory.to_str().some()?,
            "--quiet",
            "--flat-playlist",
            "--color", "no_color",
            "--print", "playlist:[playlist-started:metadata]%(id)s;%(title)s;%(webpage_url)s",
            "--print", "video:[playlist-started:url]%(url)s"
        ])?;

        let correlation_id = ::std::sync::Arc::clone(&self.id_generator).generate();

        let playlist = ::tokio::spawn({
            let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();
            let correlation_id = correlation_id.clone();

            let playlist_download_events_tx = playlist_download_events_tx.clone();
            let diagnostic_events_tx = diagnostic_events_tx.clone();

            async move {
                let (playlist, _) = ::tokio::try_join!(
                    async {
                        let payload = PlaylistDownloadStartedEvent::from_lines(stdout).await.some()?;
                        let playlist = payload.playlist.clone();

                        let event = Event {
                            metadata: EventMetadata {
                                worker_id: worker_id.clone(),
                                correlation_id: correlation_id.clone(),
                                timestamp: ::std::time::SystemTime::now(),
                            },
                            payload: PlaylistDownloadEvent::Started(payload),
                        };

                        playlist_download_events_tx.send(event)?;
                        Ok(playlist)
                    },

                    async {
                        stderr
                            .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                            .map(|payload| Event {
                                metadata: EventMetadata {
                                    worker_id: worker_id.clone(),
                                    correlation_id: correlation_id.clone(),
                                    timestamp: ::std::time::SystemTime::now(),
                                },
                                payload,
                            })
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

        let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicUsize::new(0));
        let total = playlist.videos
            .as_deref()
            .map_or(0, |videos| videos.len());

        let resolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total)));

        let unresolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(playlist.videos.as_deref().map_or_else(|| ::std::collections::VecDeque::new(), |videos| videos.iter().cloned().collect())));
        
        let unresolved_videos_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());
        
        (0..self.configurations.workers).for_each(|_| {
            ::tokio::spawn({
                let this = ::std::sync::Arc::clone(&self);

                let playlist_download_events_tx = playlist_download_events_tx.clone();
                let video_download_events_tx = video_download_events_tx.clone();
                let diagnostic_events_tx = diagnostic_events_tx.clone();

                let worker_id = ::std::sync::Arc::clone(&this.id_generator).generate();
                let correlation_id = correlation_id.clone();

                let directory = directory.clone();
                let completed = ::std::sync::Arc::clone(&completed);
                let resolved_videos = ::std::sync::Arc::clone(&resolved_videos);
                let unresolved_videos = ::std::sync::Arc::clone(&unresolved_videos);
                let unresolved_videos_notify = ::std::sync::Arc::clone(&unresolved_videos_notify);

                async move {
                    loop {
                        let (video, is_last_worker_to_poll) = {
                            let mut unresolved_videos = unresolved_videos.lock().await;
                            let video = unresolved_videos.pop_front();
                            let is_last_worker_to_poll = unresolved_videos.is_empty();
                            (video, is_last_worker_to_poll)
                        };

                        let Some(video) = video else { break };

                        let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&this).download_video(video.url.clone(), directory.clone()).await?;

                        ::tokio::try_join!(
                            async {
                                video_download_events
                                    .map(|event| event.with_metadata(EventMetadata {
                                        worker_id: worker_id.clone(),
                                        correlation_id: correlation_id.clone(),
                                        timestamp: ::std::time::SystemTime::now(),
                                    }))
                                    .map(Ok)
                                    .try_for_each(|event| async {
                                        match event.payload {
                                            VideoDownloadEvent::Completed(ref payload) => {
                                                completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                                resolved_videos.lock().await.push(payload.video.clone());

                                                let event = Event {
                                                    metadata: EventMetadata {
                                                        worker_id: worker_id.clone(),
                                                        correlation_id: correlation_id.clone(),
                                                        timestamp: ::std::time::SystemTime::now(),
                                                    },
                                                    payload: PlaylistDownloadEvent::ProgressUpdated(
                                                        PlaylistDownloadProgressUpdatedEvent {
                                                            video: payload.video.clone(),
                                                            completed_videos: completed.load(::std::sync::atomic::Ordering::Relaxed),
                                                            total_videos,
                                                        },
                                                    ),
                                                };

                                                playlist_download_events_tx.send(event)?;
                                            },
                                            _ => {},
                                        }

                                        video_download_events_tx.send(event)?;
                                        Ok(())
                                    })
                                    .await
                            },

                            async {
                                diagnostic_events
                                    .map(|event| Event {
                                        metadata: EventMetadata {
                                            worker_id: worker_id.clone(),
                                            correlation_id: correlation_id.clone(),
                                            timestamp: ::std::time::SystemTime::now(),
                                        },
                                        payload: event.payload,
                                    })
                                    .map(Ok)
                                    .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                                    .await
                                    .map_err(::anyhow::Error::from)
                            },
                        )?;
                        
                        if is_last_worker_to_poll {
                            unresolved_videos_notify.notify_one();
                        }

                        ::tokio::time::sleep(this.configurations.cooldown).await;
                    }

                    Ok::<_, ::anyhow::Error>(())
                }
            });
        });

        ::tokio::spawn({
            let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();
            let correlation_id = correlation_id.clone();

            let playlist_download_events_tx = playlist_download_events_tx.clone();

            let unresolved_videos_notify = ::std::sync::Arc::clone(&unresolved_videos_notify);

            async move {
                unresolved_videos_notify.notified().await;

                let playlist = ResolvedPlaylist {
                    url: playlist.url,
                    id: playlist.id,
                    metadata: PlaylistMetadata {
                        title: playlist.metadata.title,
                    },
                    videos: {
                        let videos = ::std::mem::take(&mut *resolved_videos.lock().await);
                        videos.is_empty().not().then_some(videos)
                    }
                };

                let event = Event {
                    metadata: EventMetadata {
                        worker_id,
                        correlation_id,
                        timestamp: ::std::time::SystemTime::now(),
                    },
                    payload: PlaylistDownloadEvent::Completed(PlaylistDownloadCompletedEvent { playlist }),
                };

                playlist_download_events_tx.send(event)?;

                Ok::<_, ::anyhow::Error>(())
            }
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
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

        let stdout = process.stdout.take().some()?;
        let stderr = process.stderr.take().some()?;

        ::tokio::task::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stdout).lines();
            
            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| MaybeOwnedString::from(line))
                .map(Ok)
                .try_for_each(|line| async { stdout_tx.send(line) })
                .await;
        });

        ::tokio::task::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stderr).lines();
            
            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| MaybeOwnedString::from(line))
                .map(Ok)
                .try_for_each(|line| async { stderr_tx.send(line) })
                .await;
        });

        Ok((
            ::tokio_stream::wrappers::UnboundedReceiverStream::new(stdout_rx),
            ::tokio_stream::wrappers::UnboundedReceiverStream::new(stderr_rx),
        ))
    }
}

trait FromYtdlpLine {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized;
}

impl FromYtdlpLine for VideoDownloadEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        VideoDownloadProgressUpdatedEvent::from_line(&line).map(Self::ProgressUpdated)
            .or(VideoDownloadStartedEvent::from_line(&line).map(Self::Started))
            .or(VideoDownloadCompletedEvent::from_line(&line).map(Self::Completed))
    }
}

impl FromYtdlpLine for VideoDownloadStartedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(
            r"\[video-started\](?P<url>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        );

        let captures = REGEX.captures(line.as_ref())?;

        let video = PartiallyResolvedVideo {
            url: parse_attr(&captures["url"])?,
            id: parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: parse_attr(&captures["title"]),
                album: parse_attr(&captures["album"]),
                artists: parse_multivalued_attr(&captures["artist"]),
                genres: parse_multivalued_attr(&captures["genre"]),
            },
        };

        Some(Self { video })
    }
}

impl FromYtdlpLine for VideoDownloadProgressUpdatedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        const NULL_ETA: MaybeOwnedString = MaybeOwnedString::Borrowed("00:00");

        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(
            r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
        );

        let captures = REGEX.captures(line.as_ref())?;

        Some(Self {
            percentage: parse_attr(&captures["percent"])?.parse().ok()?,
            eta: parse_attr(&captures["eta"]).unwrap_or(NULL_ETA),
            size: parse_attr(&captures["size"])?,
            speed: parse_attr(&captures["speed"])?,
        })
    }
}

impl FromYtdlpLine for VideoDownloadCompletedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        use ::std::ops::Not as _;

        const PREFIX: &str = "[video-completed]";

        if line.as_ref().starts_with(PREFIX).not() {
            return None;
        }

        let 

        let video = ResolvedVideo {
            url: parse_attr(&captures["url"])?,
            id: parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: parse_attr(&captures["title"]),
                album: parse_attr(&captures["album"]),
                artists: parse_multivalued_attr(&captures["artist"]),
                genres: parse_multivalued_attr(&captures["genre"]),
            },
            path: match parse_attr(&captures["path"])? {
                MaybeOwnedString::Borrowed(path) => MaybeOwnedPath::Borrowed(::std::path::Path::new(path)),
                MaybeOwnedString::Owned(path) => MaybeOwnedPath::Owned(path.into()),
            },
        };

        Some(Self { video })
    }
}

impl FromYtdlpLine for DiagnosticEvent {
    fn from_line(line: &'static str) -> Option<Self>
    where
        Self: Sized,
    {
        // static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(
        //     r"^(?P<level>WARNING|ERROR):\s*(?P<message>.+)$"
        // );

        // let captures = REGEX.captures(line.as_ref())?;
        
        // Some(Self {
        //     level: match &captures["level"] {
        //         "WARNING" => DiagnosticLevel::Warning,
        //         "ERROR" => DiagnosticLevel::Error,
        //         _ => return None,
        //     },
        //     message: parse_attr(&captures["message"])?,
        // })
        let mut attrs = line.split(ATTRIBUTE_DELIMITER);

        let level = attrs.next()?.trim();
        let message = attrs.next()?.trim();

        Some(Self {
            level: match level {
                "WARNING" => DiagnosticLevel::Warning,
                "ERROR" => DiagnosticLevel::Error,
                _ => return None,
            },
            message: message.into(),
        })
    }
}

// #[async_trait]
// trait FromYtdlpLines: Send + Sync {
//     async fn from_lines<Lines>(lines: Lines) -> Option<Self>
//     where
//         Lines: ::futures::Stream<Item = &str> + ::core::marker::Send,
//         Line: AsRef<str>,
//         Self: Sized;
// }

// #[async_trait]
// impl FromYtdlpLines for PlaylistDownloadStartedEvent {
//     async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
//     where
//         Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
//         Line: AsRef<str>,
//         Self: Sized,        
//     {
//         use ::std::ops::Not as _;
//         use ::futures::StreamExt as _;

//         static PLAYLIST_VIDEOS_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(
//             r"\[playlist-started:url\](?P<url>[^;]+)"
//         );

//         static PLAYLIST_METADATA_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(
//             r"\[playlist-started:metadata\](?P<id>[^;]+);(?P<title>[^;]+);(?P<url>[^;]+)"
//         );

//         let mut videos = Vec::new();

//         ::futures::pin_mut!(lines);

//         while let Some(line) = lines.next().await {
//             if let Some(captures) = PLAYLIST_VIDEOS_REGEX.captures(line.as_ref()) {
//                 videos.push(UnresolvedVideo {
//                     url: parse_attr(&captures["url"])?,
//                 });

//             } else if let Some(captures) = PLAYLIST_METADATA_REGEX.captures(line.as_ref()) {
//                 let playlist = PartiallyResolvedPlaylist {
//                     url: parse_attr(&captures["url"])?,
//                     id: parse_attr(&captures["id"])?,
//                     metadata: PlaylistMetadata {
//                         title: parse_attr(&captures["title"]),
//                     },
//                     videos: videos.is_empty().not().then_some(videos),
//                 };

//                 return Some(Self { playlist })
//             }
//         }

//         None
//     }
// }

fn parse_multivalued_attr(string: &str) -> Option<Vec<MaybeOwnedString>> {
    Some(parse_attr(string)?
        .split(MULTIVALUE_DELIMITER)
        .map(normalize)
        .map(|attr| attr.to_owned().into())
        .collect())
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

const ATTRIBUTE_DELIMITER: char = ';';
const MULTIVALUE_DELIMITER: char = ',';
