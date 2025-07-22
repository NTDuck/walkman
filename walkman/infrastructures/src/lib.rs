pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::PlaylistMetadata;
use ::domain::VideoMetadata;
use ::use_cases::boundaries::Update;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
use ::use_cases::models::descriptors::PartiallyResolvedVideo;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::use_cases::models::descriptors::UnresolvedVideo;
use ::use_cases::models::events::DiagnosticEvent;
use ::use_cases::models::events::DiagnosticEventPayload;
use ::use_cases::models::events::DiagnosticLevel;
use ::use_cases::models::events::Event;
use ::use_cases::models::events::EventMetadata;
use use_cases::models::events::EventRef;
use ::use_cases::models::events::PlaylistDownloadCompletedEventPayload;
use ::use_cases::models::events::PlaylistDownloadEvent;
use ::use_cases::models::events::PlaylistDownloadEventPayload;
use ::use_cases::models::events::PlaylistDownloadProgressUpdatedEventPayload;
use ::use_cases::models::events::PlaylistDownloadStartedEventPayload;
use ::use_cases::models::events::VideoDownloadCompletedEventPayload;
use ::use_cases::models::events::VideoDownloadEvent;
use ::use_cases::models::events::VideoDownloadEventPayload;
use ::use_cases::models::events::VideoDownloadProgressUpdatedEventPayload;
use ::use_cases::models::events::VideoDownloadStartedEventPayload;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::extensions::OptionExt;

pub struct DownloadVideoView {
    progress_bars: ::indicatif::MultiProgress,
    video_progress_bar: ::indicatif::ProgressBar,
}

// TODO migrate to with_key
impl DownloadVideoView {
    pub fn new() -> Fallible<Self> {
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50} {msg}");
        
        let progress_bars = ::indicatif::MultiProgress::new();
        let video_progress_bar = progress_bars.add(::indicatif::ProgressBar::new(100)
            .with_style(PROGRESS_BAR_STYLE.clone()));

        video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", "??MiB", "??MiB/s"), "??:??"));
        video_progress_bar.set_message("??%");

        Ok(Self { progress_bars, video_progress_bar })
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
        match &event.payload {
            VideoDownloadEventPayload::Started(payload) => self.update(&event.with_payload(payload)).await,
            VideoDownloadEventPayload::ProgressUpdated(payload) => self.update(&event.with_payload(payload)).await,
            VideoDownloadEventPayload::Completed(payload) => self.update(&event.with_payload(payload)).await,
        }
    }
}

#[async_trait]
impl<'event> Update<EventRef<'event, VideoDownloadStartedEventPayload>> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, VideoDownloadStartedEventPayload>) -> Fallible<()> {
        use ::colored::Colorize as _;

        let VideoDownloadStartedEventPayload { video } = event.payload;

        self.video_progress_bar
            .println(format!("Downloading video: {}", video.metadata.title.white().bold()));

        Ok(())
    }
}

#[async_trait]
impl<'event> Update<EventRef<'event, VideoDownloadProgressUpdatedEventPayload>> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, VideoDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEventPayload { percentage, size, speed, eta } = event.payload;

        self.video_progress_bar.set_position(*percentage as u64);
        self.video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", size, speed), eta));
        self.video_progress_bar.set_message(format!("{}%", percentage));

        Ok(())
    }
}

#[async_trait]
impl<'event> Update<EventRef<'event, VideoDownloadCompletedEventPayload>> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, _: &EventRef<'event, VideoDownloadCompletedEventPayload>) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50.green} {msg}");
        
        self.video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.video_progress_bar.set_prefix(self.video_progress_bar.prefix().green().to_string());
        self.video_progress_bar.set_message(self.video_progress_bar.message().green().to_string());

        self.video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{msg}");

        let DiagnosticEventPayload { message, level } = &event.payload;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

pub struct DownloadPlaylistView {
    progress_bars: ::indicatif::MultiProgress,
    playlist_progress_bar: ::indicatif::ProgressBar,
    video_progress_bars: ::std::sync::Arc<::tokio::sync::Mutex<Vec<::indicatif::ProgressBar>>>,
}

impl DownloadPlaylistView {
    pub fn new() -> Fallible<Self> {
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50} {msg}");
        
        let progress_bars = ::indicatif::MultiProgress::new();
        let playlist_progress_bar = progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(PROGRESS_BAR_STYLE.clone()));

        playlist_progress_bar.set_prefix(format!("{:<26}", ""));
        playlist_progress_bar.set_message("??/??");

        let video_progress_bars = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::new()));

        Ok(Self { progress_bars, playlist_progress_bar, video_progress_bars })
    }
}

#[async_trait]
impl Update<PlaylistDownloadEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadEvent) -> Fallible<()> {
        match event.payload {
            PlaylistDownloadEventPayload::Started(_) => self.update(event).await,
            PlaylistDownloadEventPayload::ProgressUpdated(_) => self.update(event).await,
            PlaylistDownloadEventPayload::Completed(_) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadStartedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<PlaylistDownloadStartedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadProgressUpdatedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<PlaylistDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadCompletedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<PlaylistDownloadCompletedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
        match event.payload {
            VideoDownloadEventPayload::Started(_) => self.update(event).await,
            VideoDownloadEventPayload::ProgressUpdated(_) => self.update(event).await,
            VideoDownloadEventPayload::Completed(_) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<Event<VideoDownloadStartedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<VideoDownloadStartedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<VideoDownloadProgressUpdatedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<VideoDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<VideoDownloadCompletedEventPayload>> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &Event<VideoDownloadCompletedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{msg}");

        let DiagnosticEventPayload { message, level } = &event.payload;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

#[derive(new)]
pub struct YtdlpDownloader<CommandExecutorImpl, IdGeneratorImpl> {
    command_executor: ::std::sync::Arc<CommandExecutorImpl>,
    id_generator: ::std::sync::Arc<IdGeneratorImpl>,

    configurations: YtdlpConfigurations,
}

trait CommandExecutor: Send + Sync {
    fn execute<Program, Args>(self: ::std::sync::Arc<Self>, program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>;
}

trait IdGenerator: Send + Sync {
    fn generate(self: ::std::sync::Arc<Self>) -> MaybeOwnedString;
}

pub struct YtdlpConfigurations {
    pub workers: usize,
}

#[async_trait]
impl<CommandExecutorImpl, IdGeneratorImpl> Downloader for YtdlpDownloader<CommandExecutorImpl, IdGeneratorImpl>
where
    CommandExecutorImpl: CommandExecutor + 'static,
    IdGeneratorImpl: IdGenerator + 'static,
{
    async fn download_video(
        self: ::std::sync::Arc<Self>, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        use ::futures::StreamExt as _;
        use ::futures::TryStreamExt as _;

        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        #[rustfmt::skip]
        let (stdout, stderr) = ::std::sync::Arc::clone(&self.command_executor).execute("yt-dlp", [
            &*url,
            "--paths", &directory.to_str().some()?,
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

        let correlation_id = ::std::sync::Arc::clone(&self.id_generator).generate();

        ::tokio::spawn({
            let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();

            async move {
                ::tokio::try_join!(
                    async {
                        stdout
                            .filter_map(|line| async { VideoDownloadEventPayload::from_line(line) })
                            .map(|payload| Ok(Event {
                                metadata: EventMetadata {
                                    worker_id: worker_id.clone(),
                                    correlation_id: correlation_id.clone(),
                                    timestamp: std::time::SystemTime::now(),
                                },
                                payload,
                            }))
                            .try_for_each(|event| async { video_download_events_tx.send(event) })
                            .await
                            .map_err(::anyhow::Error::from)
                    },
                    
                    async {
                        stderr
                            .filter_map(|line| async { DiagnosticEventPayload::from_line(line) })
                            .map(|payload| Ok(Event {
                                metadata: EventMetadata {
                                    worker_id: worker_id.clone(),
                                    correlation_id: correlation_id.clone(),
                                    timestamp: std::time::SystemTime::now(),
                                },
                                payload,
                            }))
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

    async fn download_playlist(
        self: ::std::sync::Arc<Self>, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        use ::futures::StreamExt as _;
        use ::futures::TryStreamExt as _;

        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let (stdout, stderr) = ::std::sync::Arc::clone(&self.command_executor).execute("yt-dlp", [
            &*url,
            "--paths", &directory.to_str().some()?,
            "--quiet",
            "--flat-playlist",
            "--color", "no_color",
            "--print", "playlist:[playlist-started:metadata]%(id)s;%(title)s",
            "--print", "video:[playlist-started:url]%(url)s"
        ])?;

        let worker_id = ::std::sync::Arc::clone(&self.id_generator).generate();
        let correlation_id = ::std::sync::Arc::clone(&self.id_generator).generate();

        let (playlist, _) = ::tokio::try_join!(
            async {
                let payload = PlaylistDownloadStartedEventPayload::from_lines(stdout).await.some()?;
                let playlist = payload.playlist.clone();

                let event = Event {
                    metadata: EventMetadata {
                        worker_id: worker_id.clone(),
                        correlation_id: correlation_id.clone(),
                        timestamp: ::std::time::SystemTime::now(),
                    },
                    payload: PlaylistDownloadEventPayload::Started(payload),
                };

                playlist_download_events_tx.send(event)?;
                Ok(playlist)
            },

            async {
                stderr
                    .filter_map(|line| async { DiagnosticEventPayload::from_line(line) })
                    .map(|payload| Ok(Event {
                        metadata: EventMetadata {
                            worker_id: worker_id.clone(),
                            correlation_id: correlation_id.clone(),
                            timestamp: std::time::SystemTime::now(),
                        },
                        payload,
                    }))
                    .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                    .await
                    .map_err(::anyhow::Error::from)
            },
        )?;

        let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicUsize::new(0));
        let total = playlist.videos.len();

        let resolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total)));

        let unresolved_videos: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(playlist.videos.clone().into()));
        
        for _ in 0..self.configurations.workers {
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

                async move {
                    while let Some(video) = unresolved_videos.lock().await.pop_front() {
                        let (video_download_events, diagnostic_events) = ::std::sync::Arc::clone(&this).download_video(video.url.clone(), directory.clone()).await?;

                        ::tokio::try_join!(
                            async {
                                ::futures::pin_mut!(video_download_events);

                                while let Some(event) = video_download_events.next().await {
                                    match event.payload {
                                        VideoDownloadEventPayload::Completed(ref payload) => {
                                            completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                            resolved_videos.lock().await.push(payload.video.clone());

                                            let event = Event {
                                                metadata: EventMetadata {
                                                    worker_id: worker_id.clone(),
                                                    correlation_id: correlation_id.clone(),
                                                    timestamp: ::std::time::SystemTime::now(),
                                                },
                                                payload: PlaylistDownloadEventPayload::ProgressUpdated(
                                                    PlaylistDownloadProgressUpdatedEventPayload {
                                                        video: payload.video.clone(),
                                                        completed: completed.load(::std::sync::atomic::Ordering::Relaxed),
                                                        total,
                                                    },
                                                ),
                                            };

                                            playlist_download_events_tx.send(event)?;
                                        },
                                        _ => {},
                                    }

                                    video_download_events_tx.send(event)?;
                                }

                                Ok::<_, ::anyhow::Error>(())
                            },

                            async {
                                ::futures::pin_mut!(diagnostic_events);

                                while let Some(event) = diagnostic_events.next().await {
                                    let event = Event {
                                        metadata: EventMetadata {
                                            worker_id: worker_id.clone(),
                                            correlation_id: correlation_id.clone(),
                                            timestamp: ::std::time::SystemTime::now(),
                                        },
                                        payload: event.payload,
                                    };

                                    diagnostic_events_tx.send(event)?;
                                }

                                Ok::<_, ::anyhow::Error>(())
                            },
                        )?;
                    }

                    Ok::<_, ::anyhow::Error>(())
                }
            });
        }

        let playlist = ResolvedPlaylist {
            url: playlist.url,
            id: playlist.id,
            metadata: PlaylistMetadata {
                title: playlist.metadata.title,
            },
            videos: ::std::mem::take(&mut *resolved_videos.lock().await),
        };

        let event = Event {
            metadata: EventMetadata {
                worker_id: ::std::sync::Arc::clone(&self.id_generator).generate(),
                correlation_id: correlation_id.clone(),
                timestamp: ::std::time::SystemTime::now(),
            },
            payload: PlaylistDownloadEventPayload::Completed(PlaylistDownloadCompletedEventPayload { playlist }),
        };

        playlist_download_events_tx.send(event)?;

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

#[derive(new)]
pub struct TokioCommandExecutor;

impl CommandExecutor for TokioCommandExecutor {
    fn execute<Program, Args>(self: ::std::sync::Arc<Self>, program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>,
        Self: Sized,
    {
        use ::tokio::io::AsyncBufReadExt as _;
        use ::futures::StreamExt as _;

        let (stdout_tx, stdout_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let mut process = ::tokio::process::Command::new(program)
            .args(args)
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        ::tokio::task::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stdout).lines();
            
            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| MaybeOwnedString::from(line))
                .for_each(|line| async { stdout_tx.send(line).unwrap() })
                .await;
        });

        ::tokio::task::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stderr).lines();
            
            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| MaybeOwnedString::from(line))
                .for_each(|line| async { stderr_tx.send(line).unwrap() })
                .await;
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stdout_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stderr_rx)),
        ))
    }
}

#[derive(new)]
pub struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn generate(self: ::std::sync::Arc<Self>) -> MaybeOwnedString {
        ::uuid::Uuid::new_v4().to_string().into()
    }
}

trait FromYtdlpLine {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized;
}

impl FromYtdlpLine for VideoDownloadEventPayload {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        VideoDownloadProgressUpdatedEventPayload::from_line(&line).map(Self::ProgressUpdated)
            .or(VideoDownloadStartedEventPayload::from_line(&line).map(Self::Started))
            .or(VideoDownloadCompletedEventPayload::from_line(&line).map(Self::Completed))
    }
}

impl FromYtdlpLine for VideoDownloadStartedEventPayload {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-started\](?P<url>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        );

        let captures = REGEX.captures(line.as_ref())?;

        let video = PartiallyResolvedVideo {
            url: parse_attr(&captures["url"])?,
            id: parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: parse_attr(&captures["title"])?,
                album: parse_attr(&captures["album"])?,
                artists: parse_multivalued_attr(&captures["artist"]),
                genres: parse_multivalued_attr(&captures["genre"]),
            },
        };

        Some(Self { video })
    }
}

impl FromYtdlpLine for VideoDownloadProgressUpdatedEventPayload {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
        );

        let captures = REGEX.captures(line.as_ref())?;

        Some(Self {
            percentage: parse_attr(&captures["percent"])?.parse().ok()?,
            eta: parse_attr(&captures["eta"]).unwrap_or("00:00".into()),
            size: parse_attr(&captures["size"])?,
            speed: parse_attr(&captures["speed"])?,
        })
    }
}

impl FromYtdlpLine for VideoDownloadCompletedEventPayload {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-completed\](?P<url>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+);(?P<path>[^;]+)"
        );

        let captures = REGEX.captures(line.as_ref())?;

        let video = ResolvedVideo {
            url: parse_attr(&captures["url"])?,
            id: parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: parse_attr(&captures["title"])?,
                album: parse_attr(&captures["album"])?,
                artists: parse_multivalued_attr(&captures["artist"]),
                genres: parse_multivalued_attr(&captures["genre"]),
            },
            path: ::std::path::PathBuf::from(&*parse_attr(&captures["path"])?).into(),
        };

        Some(Self { video })
    }
}

impl FromYtdlpLine for DiagnosticEventPayload {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"^(?P<level>WARNING|ERROR):\s*(?P<message>.+)$"
        );

        let captures = REGEX.captures(line.as_ref())?;
        
        Some(Self {
            level: match &captures["level"] {
                "WARNING" => DiagnosticLevel::Warning,
                "ERROR" => DiagnosticLevel::Error,
                _ => return None,
            },
            message: parse_attr(&captures["message"])?,
        })
    }
}

#[async_trait]
trait FromYtdlpLines: Send + Sync {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized;
}

#[async_trait]
impl FromYtdlpLines for PlaylistDownloadStartedEventPayload {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized,        
    {
        use ::futures::StreamExt as _;

        static PLAYLIST_VIDEOS_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[playlist-started:url\];(?P<url>[^;]+)"
        );

        static PLAYLIST_METADATA_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[playlist-started:metadata\];(?P<id>[^;]+);(?P<title>[^;]+);(?P<url>[^;]+)"
        );

        let mut videos = Vec::new();

        ::futures::pin_mut!(lines);

        while let Some(line) = lines.next().await {
            if let Some(captures) = PLAYLIST_VIDEOS_REGEX.captures(line.as_ref()) {
                videos.push(UnresolvedVideo {
                    url: parse_attr(&captures["url"])?,
                });

            } else if let Some(captures) = PLAYLIST_METADATA_REGEX.captures(line.as_ref()) {
                let playlist = PartiallyResolvedPlaylist {
                    url: parse_attr(&captures["url"])?,
                    id: parse_attr(&captures["id"])?,
                    metadata: PlaylistMetadata {
                        title: parse_attr(&captures["title"])?,
                    },
                    videos,
                };

                return Some(Self { playlist })
            }
        }

        None
    }
}

fn parse_multivalued_attr(string: &str) -> Vec<MaybeOwnedString> {
    match parse_attr(string) {
        Some(attrs) => attrs
            .split(',')
            .map(normalize)
            .map(|attr| attr.to_owned().into())
            .collect(),
        None => Vec::new(),
    }
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

#[derive(new)]
pub struct GenericMetadataWriter;

#[async_trait]
impl MetadataWriter for GenericMetadataWriter {
    async fn write_video(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        use ::lofty::file::TaggedFileExt as _;
        use ::lofty::tag::Accessor as _;
        use ::lofty::tag::TagExt as _;

        let mut file = ::lofty::read_from_path(&video.path)?;

        let tag = match file.primary_tag_mut() {
            Some(tag) => tag,
            None => match file.first_tag_mut() {
                Some(tag) => tag,
                None => {
                    file.insert_tag(::lofty::tag::Tag::new(file.primary_tag_type()));
                    file.primary_tag_mut().unwrap()
                },
            },
        };

        tag.set_title(video.metadata.title.to_owned().into());
        tag.set_album(video.metadata.album.to_owned().into());
        tag.set_artist(video.metadata.artists.join(", ").to_owned().into());
        tag.set_genre(video.metadata.genres.join(", ").to_owned().into());

        tag.save_to_path(&video.path, ::lofty::config::WriteOptions::default())?;

        Ok(())
    }
}

#[derive(new)]
pub struct Id3MetadataWriter;

#[async_trait]
impl MetadataWriter for Id3MetadataWriter {
    async fn write_video(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        use ::id3::TagLike as _;

        let mut tag = ::id3::Tag::new();

        tag.set_title(&*video.metadata.title);
        tag.set_album(&*video.metadata.album);
        tag.set_artist(&*video.metadata.artists.join(", "));
        tag.set_genre(&*video.metadata.genres.join(", "));

        tag.write_to_path(&video.path, ::id3::Version::Id3v23)?;

        Ok(())
    }
}

#[derive(new)]
pub struct Id3PlaylistAsAlbumMetadataWriter;

#[async_trait]
impl MetadataWriter for Id3PlaylistAsAlbumMetadataWriter {
    async fn write_video(self: ::std::sync::Arc<Self>, video: &ResolvedVideo) -> Fallible<()> {
        self.write_video(video, None)
    }

    async fn write_playlist(self: ::std::sync::Arc<Self>, playlist: &ResolvedPlaylist) -> Fallible<()> {
        use ::rayon::iter::IntoParallelRefIterator as _;
        use ::rayon::iter::ParallelIterator as _;

        playlist.videos
            .par_iter()
            .try_for_each(|video| ::std::sync::Arc::clone(&self).write_video(video, Some(playlist)))
    }
}

impl Id3PlaylistAsAlbumMetadataWriter {
    fn write_video(self: ::std::sync::Arc<Self>, video: &ResolvedVideo, playlist: Option<&ResolvedPlaylist>) -> Fallible<()> {
        use ::id3::TagLike as _;
        
        let mut tag = ::id3::Tag::new();

        tag.set_title(&*video.metadata.title);
        tag.set_artist(&*video.metadata.artists.join(", "));
        tag.set_genre(&*video.metadata.genres.join(", "));

        if let Some(playlist) = playlist {
            tag.set_album(&*playlist.metadata.title);
        }

        tag.write_to_path(&video.path, ::id3::Version::Id3v23)?;

        Ok(())
    }
}
