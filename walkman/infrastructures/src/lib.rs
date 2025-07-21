pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use domain::PlaylistMetadata;
use domain::VideoMetadata;
use futures::FutureExt;
use ::use_cases::boundaries::Update;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use use_cases::models::descriptors::PartiallyResolvedPlaylist;
use use_cases::models::descriptors::PartiallyResolvedVideo;
use use_cases::models::descriptors::ResolvedPlaylist;
use use_cases::models::descriptors::ResolvedVideo;
use use_cases::models::descriptors::UnresolvedVideo;
use use_cases::models::events::DiagnosticEvent;
use use_cases::models::events::DiagnosticEventPayload;
use use_cases::models::events::DiagnosticLevel;
use use_cases::models::events::Event;
use use_cases::models::events::EventMetadata;
use use_cases::models::events::PlaylistDownloadCompletedEventPayload;
use use_cases::models::events::PlaylistDownloadEvent;
use use_cases::models::events::PlaylistDownloadEventPayload;
use use_cases::models::events::PlaylistDownloadProgressUpdatedEventPayload;
use use_cases::models::events::PlaylistDownloadStartedEventPayload;
use use_cases::models::events::VideoDownloadCompletedEventPayload;
use use_cases::models::events::VideoDownloadEvent;
use use_cases::models::events::VideoDownloadEventPayload;
use use_cases::models::events::VideoDownloadProgressUpdatedEventPayload;
use use_cases::models::events::VideoDownloadStartedEventPayload;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

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
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        match event.payload {
            VideoDownloadEventPayload::Started(_) => self.update(event).await,
            VideoDownloadEventPayload::ProgressUpdated(_) => self.update(event).await,
            VideoDownloadEventPayload::Completed(_) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<Event<VideoDownloadStartedEventPayload>> for DownloadVideoView {
    async fn update(&self, event: &Event<VideoDownloadStartedEventPayload>) -> Fallible<()> {
        use ::colored::Colorize as _;

        let VideoDownloadStartedEventPayload { video } = &event.payload;

        self.video_progress_bar
            .println(format!("Downloading video: {}", video.metadata.title.white().bold()));

        Ok(())
    }
}

#[async_trait]
impl Update<Event<VideoDownloadProgressUpdatedEventPayload>> for DownloadVideoView {
    async fn update(&self, event: &Event<VideoDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEventPayload { percentage, size, speed, eta } = &event.payload;

        self.video_progress_bar.set_position(*percentage as u64);
        self.video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", size, speed), eta));
        self.video_progress_bar.set_message(format!("{}%", percentage));

        Ok(())
    }
}

#[async_trait]
impl Update<Event<VideoDownloadCompletedEventPayload>> for DownloadVideoView {
    async fn update(&self, _: &Event<VideoDownloadCompletedEventPayload>) -> Fallible<()> {
        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50.green} {msg}");
        
        self.video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadVideoView {
    async fn update(&self, event: &DiagnosticEvent) -> Fallible<()> {
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
    async fn update(&self, event: &PlaylistDownloadEvent) -> Fallible<()> {
        match event.payload {
            PlaylistDownloadEventPayload::Started(_) => self.update(event).await,
            PlaylistDownloadEventPayload::ProgressUpdated(_) => self.update(event).await,
            PlaylistDownloadEventPayload::Completed(_) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadStartedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<PlaylistDownloadStartedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadProgressUpdatedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<PlaylistDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<PlaylistDownloadCompletedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<PlaylistDownloadCompletedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadPlaylistView {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        match event.payload {
            VideoDownloadEventPayload::Started(_) => self.update(event).await,
            VideoDownloadEventPayload::ProgressUpdated(_) => self.update(event).await,
            VideoDownloadEventPayload::Completed(_) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<Event<VideoDownloadStartedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<VideoDownloadStartedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<VideoDownloadProgressUpdatedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<VideoDownloadProgressUpdatedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<Event<VideoDownloadCompletedEventPayload>> for DownloadPlaylistView {
    async fn update(&self, event: &Event<VideoDownloadCompletedEventPayload>) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadPlaylistView {
    async fn update(&self, event: &DiagnosticEvent) -> Fallible<()> {
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
#[derive(Debug, Clone)]
pub struct YtdlpDownloader<CommandExecutorImpl, IdGeneratorImpl, ParserImpl> {
    command_executor: ::std::sync::Arc<CommandExecutorImpl>,
    id_generator: ::std::sync::Arc<IdGeneratorImpl>,
    parser: ::std::sync::Arc<ParserImpl>,

    configurations: YtdlpConfigurations,
}

trait CommandExecutor: Send + Sync {
    fn execute<Program, Args>(&self, program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>;
}

trait IdGenerator: Send + Sync {
    fn generate(&self) -> MaybeOwnedString;
}

trait Parser: LineParser<VideoDownloadEventPayload> + LineParser<DiagnosticEventPayload> {}

impl<T> Parser for T
where T: LineParser<VideoDownloadEventPayload> + LineParser<DiagnosticEventPayload>,
{
}

trait LineParser<T>: Send + Sync {
    fn parse<Line>(&self, line: Line) -> Option<T>
    where
        T: Sized,
        Line: AsRef<str>;    
}

trait LinesParser<T>: Send + Sync {
    fn parse<Lines, Line>(&self, lines: Lines) -> Option<T>
    where
        T: Sized,
        Lines: IntoIterator<Item = Line>,
        Line: AsRef<str>;
}

#[derive(Debug, Clone, Copy)]
pub struct YtdlpConfigurations {
    pub workers: usize,
}

#[async_trait]
impl<CommandExecutorImpl, IdGeneratorImpl, ParserImpl> Downloader for YtdlpDownloader<CommandExecutorImpl, IdGeneratorImpl, ParserImpl>
where
    CommandExecutorImpl: CommandExecutor + 'static,
    IdGeneratorImpl: IdGenerator + 'static,
    ParserImpl: Parser + 'static,
{
    async fn download_video(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        #[rustfmt::skip]
        let (stdout, stderr) = self.command_executor.execute("yt-dlp", [
            &*url,
            "--paths", &directory.to_string_lossy(),
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
            "--print", "before_dl:[video-started]%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            "--progress-template", "[video-downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s",
            "--print", "after_move:[video-completed]%(filepath)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
        ])?;

        let worker_id = self.id_generator.generate();
        let correlation_id = self.id_generator.generate();

        ::tokio::task::spawn({
            let parser = self.parser.clone();

            let worker_id = worker_id.clone();
            let correlation_id = correlation_id.clone();

            async move {
                use ::futures::StreamExt as _;

                stdout
                    .filter_map(|line| async { parser.parse(line) })
                    .map(|payload| Event {
                        metadata: EventMetadata {
                            worker_id: worker_id.clone(),
                            correlation_id: correlation_id.clone(),
                            timestamp: ::std::time::SystemTime::now(),
                        },
                        payload,
                    })
                    .for_each(|event| async { video_download_events_tx.send(event).unwrap() })
                    .await;
            }
        });

        ::tokio::task::spawn({
            let parser = self.parser.clone();

            let worker_id = worker_id.clone();
            let correlation_id = correlation_id.clone();

            async move {
                use ::futures::StreamExt as _;

                stderr
                    .filter_map(|line| async { parser.parse(line) })
                    .map(|payload| Event {
                        metadata: EventMetadata {
                            worker_id: worker_id.clone(),
                            correlation_id: correlation_id.clone(),
                            timestamp: ::std::time::SystemTime::now(),
                        },
                        payload,
                    })
                    .for_each(|event| async { diagnostic_events_tx.send(event).unwrap() })
                    .await;
            }
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }

    async fn download_playlist(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        // use ::std::io::BufRead as _;
        // use crate::private::FromYtDlpVideoDownloadOutput as _;
        // use crate::private::FromYtDlpPlaylistDownloadOutput as _;

        // let (playlist_event_stream_tx, mut playlist_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();
        // let playlist_event_stream_tx = ::std::sync::Arc::new(playlist_event_stream_tx);

        // let (video_event_stream_txs, video_event_stream_rxs): (Vec<_>, Vec<_>) = (0..self.configurations.workers)
        //     .map(|_| ::tokio::sync::mpsc::unbounded_channel())
        //     .map(|(tx, rx)| (::std::sync::Arc::new(tx), rx))
        //     .unzip();

        // let (diagnostic_event_stream_tx, mut diagnostic_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();
        // let diagnostic_event_stream_tx = ::std::sync::Arc::new(diagnostic_event_stream_tx);

        // #[rustfmt::skip]
        // let mut process = ::std::process::Command::new("yt-dlp")
        //     .args([
        //         &*url,
        //         "--paths", &directory.to_str().unwrap(),
        //         "--quiet",
        //         "--flat-playlist",
        //         "--color", "no_color",
        //         "--print", "playlist:[playlist-started:metadata]%(id)s;%(title)s",
        //         "--print", "video:[playlist-started:url]%(url)s"
        //     ])
        //     .stdout(::std::process::Stdio::piped())
        //     .stderr(::std::process::Stdio::piped())
        //     .spawn()?;

        // let stdout = process.stdout.take().unwrap();
        // let stderr = process.stderr.take().unwrap();
        
        // ::tokio::task::spawn_blocking({
        //     let diagnostic_event_stream_tx = diagnostic_event_stream_tx.clone();

        //     move || {
        //         let reader = ::std::io::BufReader::new(stderr);

        //         reader.lines()
        //             .filter_map(|line| line.ok())
        //             .filter_map(|line| DownloadDiagnosticEvent::from_line(&line))
        //             .try_for_each(|event| diagnostic_event_stream_tx.send(event))
        //     }
        // });

        // let playlist = ::tokio::task::spawn_blocking({
        //     let playlist_event_stream_tx = playlist_event_stream_tx.clone();

        //     move || {
        //         let reader = ::std::io::BufReader::new(stdout);

        //         let lines = reader.lines()
        //             .filter_map(|line| line.ok());

        //         let event = PlaylistDownloadStartedEvent::from_lines(lines).unwrap();
        //         let playlist = event.playlist.clone();

        //         playlist_event_stream_tx.send(PlaylistDownloadEvent::Started(event))?;

        //         Ok::<_, ::anyhow::Error>(playlist)
        //     }
        // }).await??;

        // let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicUsize::new(0));
        // let total = playlist.metadata.video_urls.len();

        // let playlist_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total)));

        // let playlist_video_urls: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
        //     ::std::sync::Arc::new(::tokio::sync::Mutex::new(playlist.metadata.video_urls.clone().into()));

        // for index in 0..self.configurations.workers {
        //     ::tokio::spawn({
        //         let this = self.clone();

        //         let playlist_event_stream_tx = playlist_event_stream_tx.clone();
        //         let video_event_stream_tx = video_event_stream_txs[index].clone();
        //         let diagnostic_event_stream_tx = diagnostic_event_stream_tx.clone();

        //         let directory = directory.clone();
        //         let completed = completed.clone();

        //         let playlist_videos = playlist_videos.clone();
        //         let playlist_video_urls = playlist_video_urls.clone();

        //         async move {
        //             while let Some(playlist_video_url) = playlist_video_urls.lock().await.pop_front() {
        //                 let (video_event_stream, video_diagnostic_event_stream) = this.download_video(playlist_video_url, directory.clone()).await?;

        //                 ::tokio::try_join!(
        //                     async {
        //                         use ::futures_util::StreamExt as _;

        //                         ::futures_util::pin_mut!(video_event_stream);

        //                         while let Some(event) = video_event_stream.next().await {
        //                             match event {
        //                                 VideoDownloadPayload::Completed(VideoDownloadCompletedEvent { ref video }) => {
        //                                     completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
        //                                     playlist_videos.lock().await.push(video.clone());

        //                                     let event = PlaylistDownloadProgressUpdatedEvent {
        //                                         video: video.clone(),

        //                                         completed: completed.load(::std::sync::atomic::Ordering::Relaxed),
        //                                         total,
        //                                     };

        //                                     playlist_event_stream_tx.send(PlaylistDownloadEvent::ProgressUpdated(event))?;
        //                                 },
        //                                 _ => {},
        //                             }

        //                             video_event_stream_tx.send(event)?;
        //                         }

        //                         Ok::<_, ::anyhow::Error>(())
        //                     },

        //                     async {
        //                         use ::futures_util::StreamExt as _;
                            
        //                         ::futures_util::pin_mut!(video_diagnostic_event_stream);

        //                         while let Some(event) = video_diagnostic_event_stream.next().await {
        //                             diagnostic_event_stream_tx.send(event)?;
        //                         }

        //                         Ok::<_, ::anyhow::Error>(())
        //                     },
        //                 )?;
        //             }

        //             Ok::<_, ::anyhow::Error>(())
        //         }
        //     });
        // }

        // let playlist = Playlist {
        //     id: playlist.id,
        //     metadata: playlist.metadata,
        //     videos: ::std::mem::take(&mut *playlist_videos.lock().await),
        // };

        // let event = PlaylistDownloadCompletedEvent { playlist };
        // playlist_event_stream_tx.send(PlaylistDownloadEvent::Completed(event))?;

        // let playlist_event_stream = ::async_stream::stream! {
        //     while let Some(event) = playlist_event_stream_rx.recv().await {
        //         yield event;
        //     }
        // };

        // let video_event_streams = video_event_stream_rxs
        //     .into_iter()
        //     .map(|mut video_event_stream_rx| ::async_stream::stream! {
        //         while let Some(event) = video_event_stream_rx.recv().await {
        //             yield event;
        //         }
        //     })
        //     .map(|stream| ::std::boxed::Box::pin(stream) as BoxedStream<VideoDownloadPayload>)
        //     .collect::<Vec<_>>()
        //     .into_boxed_slice();

        // let diagnostic_event_stream = ::async_stream::stream! {
        //     while let Some(event) = diagnostic_event_stream_rx.recv().await {
        //         yield event;
        //     }
        // };

        // Ok((
        //     ::std::boxed::Box::pin(playlist_event_stream),
        //     video_event_streams,
        //     ::std::boxed::Box::pin(diagnostic_event_stream),
        // ))

        Err(::anyhow::anyhow!("Playlist downloading is not implemented yet."))
    }
}

struct TokioCommandExecutor;

impl CommandExecutor for TokioCommandExecutor {
    fn execute<Program, Args>(&self, program: Program, args: Args) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
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

        #[rustfmt::skip]
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

struct UuidGenerator;

impl IdGenerator for UuidGenerator {
    fn generate(&self) -> MaybeOwnedString {
        ::uuid::Uuid::new_v4().to_string().into()
    }
}

struct YtdlpParser;

impl<T> LineParser<T> for YtdlpParser
where
    T: FromYtdlpLine,
{
    fn parse<Line>(&self, line: Line) -> Option<T>
    where
        T: Sized,
        Line: AsRef<str>,
    {
        T::from_line(line)
    }
}

impl<T> LinesParser<T> for YtdlpParser
where
    T: FromYtdlpLines,
{
    fn parse<Lines, Line>(&self, lines: Lines) -> Option<T>
    where
        T: Sized,
        Lines: IntoIterator<Item = Line>,
        Line: AsRef<str>,
    {
        T::from_lines(lines)
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
            url: YtdlpParser::parse_attr(&captures["url"])?,
            id: YtdlpParser::parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: YtdlpParser::parse_attr(&captures["title"])?,
                album: YtdlpParser::parse_attr(&captures["album"])?,
                artists: YtdlpParser::parse_multivalued_attr(&captures["artist"]),
                genres: YtdlpParser::parse_multivalued_attr(&captures["genre"]),
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
            percentage: YtdlpParser::parse_attr(&captures["percent"])?.parse().ok()?,
            eta: YtdlpParser::parse_attr(&captures["eta"]).unwrap_or("00:00".into()),
            size: YtdlpParser::parse_attr(&captures["size"])?,
            speed: YtdlpParser::parse_attr(&captures["speed"])?,
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
            url: YtdlpParser::parse_attr(&captures["url"])?,
            id: YtdlpParser::parse_attr(&captures["id"])?,
            metadata: VideoMetadata {
                title: YtdlpParser::parse_attr(&captures["title"])?,
                album: YtdlpParser::parse_attr(&captures["album"])?,
                artists: YtdlpParser::parse_multivalued_attr(&captures["artist"]),
                genres: YtdlpParser::parse_multivalued_attr(&captures["genre"]),
            },
            path: ::std::path::PathBuf::from(&*YtdlpParser::parse_attr(&captures["path"])?).into(),
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
            message: YtdlpParser::parse_attr(&captures["message"])?,
        })
    }
}

trait FromYtdlpLines {
    fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: IntoIterator<Item = Line>,
        Line: AsRef<str>,
        Self: Sized;
}

impl FromYtdlpLines for PlaylistDownloadStartedEventPayload {
    fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: IntoIterator<Item = Line>,
        Line: AsRef<str>,
        Self: Sized,        
    {
        static PLAYLIST_VIDEOS_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[playlist-started:url\];(?P<url>[^;]+)"
        );

        static PLAYLIST_METADATA_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[playlist-started:metadata\];(?P<id>[^;]+);(?P<title>[^;]+);(?P<url>[^;]+)"
        );

        let mut videos = Vec::new();

        for line in lines {
            if let Some(captures) = PLAYLIST_VIDEOS_REGEX.captures(line.as_ref()) {
                videos.push(UnresolvedVideo {
                    url: YtdlpParser::parse_attr(&captures["url"])?,
                });

            } else if let Some(captures) = PLAYLIST_METADATA_REGEX.captures(line.as_ref()) {
                let playlist = PartiallyResolvedPlaylist {
                    url: YtdlpParser::parse_attr(&captures["url"])?,
                    id: YtdlpParser::parse_attr(&captures["id"])?,
                    metadata: PlaylistMetadata {
                        title: YtdlpParser::parse_attr(&captures["title"])?,
                    },
                    videos,
                };

                return Some(Self { playlist })
            }
        }

        None
    }
}

impl YtdlpParser {
    fn parse_multivalued_attr(string: &str) -> Vec<MaybeOwnedString> {
        match Self::parse_attr(string) {
            Some(attrs) => attrs
                .split(',')
                .map(Self::normalize)
                .map(|attr| attr.to_owned().into())
                .collect(),
            None => Vec::new(),
        }
    }

    fn parse_attr(string: &str) -> Option<MaybeOwnedString> {
        let string = Self::normalize(string);

        if string == "NA" {
            None
        } else {
            Some(string.to_owned().into())
        }
    }

    fn normalize(string: &str) -> &str {
        string.trim()
    }
}

#[derive(new)]
pub struct GenericMetadataWriter;

#[async_trait]
impl MetadataWriter for GenericMetadataWriter {
    async fn write_video(&self, video: &ResolvedVideo) -> Fallible<()> {
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
    async fn write_video(&self, video: &ResolvedVideo) -> Fallible<()> {
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
    async fn write_video(&self, video: &ResolvedVideo) -> Fallible<()> {
        self.write_video(video, None)
    }

    async fn write_playlist(&self, playlist: &ResolvedPlaylist) -> Fallible<()> {
        use ::rayon::iter::IntoParallelRefIterator as _;
        use ::rayon::iter::ParallelIterator as _;

        playlist.videos
            .par_iter()
            .try_for_each(|video| self.write_video(video, Some(playlist)))
    }
}

impl Id3PlaylistAsAlbumMetadataWriter {
    fn write_video(&self, video: &ResolvedVideo, playlist: Option<&ResolvedPlaylist>) -> Fallible<()> {
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
