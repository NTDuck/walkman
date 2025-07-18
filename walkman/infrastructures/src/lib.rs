pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::Playlist;
use ::domain::UnresolvedVideo;
use ::domain::Video;
use ::domain::VideoMetadata;
use ::use_cases::boundaries::Update;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use ::use_cases::models::DiagnosticLevel;
use ::use_cases::models::DownloadDiagnosticEvent;
use ::use_cases::models::PlaylistDownloadCompletedEvent;
use ::use_cases::models::PlaylistDownloadEvent;
use ::use_cases::models::PlaylistDownloadProgressUpdatedEvent;
use ::use_cases::models::PlaylistDownloadStartedEvent;
use ::use_cases::models::VideoDownloadCompletedEvent;
use ::use_cases::models::VideoDownloadProgressUpdatedEvent;
use ::use_cases::models::VideoDownloadEvent;
use ::use_cases::models::VideoDownloadStartedEvent;

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
        static VIDEO_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50} {msg}");
        
        let progress_bars = ::indicatif::MultiProgress::new();
        let video_progress_bar = progress_bars.add(::indicatif::ProgressBar::new(100)
            .with_style(VIDEO_PROGRESS_BAR_STYLE.clone()));

        video_progress_bar.set_prefix(format!("{:>10} @ {:>10} {:>4}", "??MiB", "??MiB/s", "??:??"));
        video_progress_bar.set_message("??%");

        Ok(Self { progress_bars, video_progress_bar })
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadVideoView {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        match event {
            VideoDownloadEvent::Started(event) => self.update(event).await,
            VideoDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            VideoDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<DownloadDiagnosticEvent> for DownloadVideoView {
    async fn update(&self, event: &DownloadDiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let message = match event.level {
            DiagnosticLevel::Warning => event.message.yellow(),
            DiagnosticLevel::Error => event.message.red(),
        };

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{msg}");

        let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(message.to_string());

        Ok(())
    }
}

pub struct DownloadPlaylistView {
    progress_bars: ::indicatif::MultiProgress,
}

impl DownloadPlaylistView {
    pub fn new() -> Fallible<Self> {
        let playlist_progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")?;
        let playlist_progress_bar = ::indicatif::ProgressBar::new(100).with_style(playlist_progress_bar_style);

        playlist_progress_bar.set_prefix(format!("{:>10} {:>10} {:>4}", "??MiB", "??MiB/s", "??:??"));
        playlist_progress_bar.set_message("??/??");

        let progress_bars = ::indicatif::MultiProgress::new();
        progress_bars.add(playlist_progress_bar);

        Ok(Self { progress_bars })
    }
}

#[async_trait]
impl Update<PlaylistDownloadEvent> for DownloadPlaylistView {
    async fn update(&self, event: &PlaylistDownloadEvent) -> Fallible<()> {
        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadPlaylistView {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        Ok(())
    }
}

#[async_trait]
impl Update<DownloadDiagnosticEvent> for DownloadPlaylistView {
    async fn update(&self, event: &DownloadDiagnosticEvent) -> Fallible<()> {
        Ok(())
    }
}

#[derive(new)]
#[derive(Clone)]
pub struct YtDlpDownloader {
    configurations: YtDlpDownloaderConfigurations,
}

#[derive(Clone)]
pub struct YtDlpDownloaderConfigurations {
    pub concurrent_video_downloads: usize,
}

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DownloadDiagnosticEvent>)> {
        use ::std::io::BufRead as _;
        use crate::private::FromYtDlpVideoDownloadOutput as _;

        let (video_event_stream_tx, mut video_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_event_stream_tx, mut diagnostic_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();

        #[rustfmt::skip]
        let mut process = ::std::process::Command::new("yt-dlp")
            .args([
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
            ])
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        ::tokio::task::spawn_blocking(move || {
            let reader = ::std::io::BufReader::new(stdout);

            reader.lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| VideoDownloadEvent::from_line(&line))
                .try_for_each(|event| video_event_stream_tx.send(event))
        });

        ::tokio::task::spawn_blocking(move || {
            let reader = ::std::io::BufReader::new(stderr);

            reader.lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| DownloadDiagnosticEvent::from_line(&line))
                .try_for_each(|event| diagnostic_event_stream_tx.send(event))
        });

        let video_event_stream = ::async_stream::stream! {
            while let Some(event) = video_event_stream_rx.recv().await {
                yield event;
            }
        };

        let diagnostic_event_stream = ::async_stream::stream! {
            while let Some(event) = diagnostic_event_stream_rx.recv().await {
                yield event;
            }
        };

        Ok((
            ::std::boxed::Box::pin(video_event_stream),
            ::std::boxed::Box::pin(diagnostic_event_stream),
        ))
    }

    async fn download_playlist(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, ::std::boxed::Box<[BoxedStream<VideoDownloadEvent>]>, BoxedStream<DownloadDiagnosticEvent>)> {
        use ::std::io::BufRead as _;
        use crate::private::FromYtDlpVideoDownloadOutput as _;
        use crate::private::FromYtDlpPlaylistDownloadOutput as _;

        let (playlist_event_stream_tx, mut playlist_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let playlist_event_stream_tx = ::std::sync::Arc::new(playlist_event_stream_tx);

        let (video_event_stream_txs, video_event_stream_rxs): (Vec<_>, Vec<_>) = (0..self.configurations.concurrent_video_downloads)
            .map(|_| ::tokio::sync::mpsc::unbounded_channel())
            .map(|(tx, rx)| (::std::sync::Arc::new(tx), rx))
            .unzip();

        let (diagnostic_event_stream_tx, mut diagnostic_event_stream_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let diagnostic_event_stream_tx = ::std::sync::Arc::new(diagnostic_event_stream_tx);

        #[rustfmt::skip]
        let mut process = ::std::process::Command::new("yt-dlp")
            .args([
                &*url,
                "--paths", &directory.to_str().unwrap(),
                "--quiet",
                "--flat-playlist",
                "--color", "no_color",
                "--print", "playlist:[playlist-started:metadata]%(id)s;%(title)s",
                "--print", "video:[playlist-started:url]%(url)s"
            ])
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();
        
        ::tokio::task::spawn_blocking({
            let diagnostic_event_stream_tx = diagnostic_event_stream_tx.clone();

            move || {
                let reader = ::std::io::BufReader::new(stderr);

                reader.lines()
                    .filter_map(|line| line.ok())
                    .filter_map(|line| DownloadDiagnosticEvent::from_line(&line))
                    .try_for_each(|event| diagnostic_event_stream_tx.send(event))
            }
        });

        let playlist = ::tokio::task::spawn_blocking({
            let playlist_event_stream_tx = playlist_event_stream_tx.clone();

            move || {
                let reader = ::std::io::BufReader::new(stdout);

                let lines = reader.lines()
                    .filter_map(|line| line.ok());

                let event = PlaylistDownloadStartedEvent::from_lines(lines).unwrap();
                let playlist = event.playlist.clone();

                playlist_event_stream_tx.send(PlaylistDownloadEvent::Started(event))?;

                Ok::<_, ::anyhow::Error>(playlist)
            }
        }).await??;

        let completed = ::std::sync::Arc::new(::std::sync::atomic::AtomicUsize::new(0));
        let total = playlist.metadata.video_urls.len();

        let playlist_videos: ::std::sync::Arc<::tokio::sync::Mutex<Vec<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total)));

        let playlist_video_urls: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::VecDeque<_>>> =
            ::std::sync::Arc::new(::tokio::sync::Mutex::new(playlist.metadata.video_urls.clone().into()));

        for index in 0..self.configurations.concurrent_video_downloads {
            ::tokio::spawn({
                let this = self.clone();

                let playlist_event_stream_tx = playlist_event_stream_tx.clone();
                let video_event_stream_tx = video_event_stream_txs[index].clone();
                let diagnostic_event_stream_tx = diagnostic_event_stream_tx.clone();

                let directory = directory.clone();
                let completed = completed.clone();

                let playlist_videos = playlist_videos.clone();
                let playlist_video_urls = playlist_video_urls.clone();

                async move {
                    while let Some(playlist_video_url) = playlist_video_urls.lock().await.pop_front() {
                        let (video_event_stream, video_diagnostic_event_stream) = this.download_video(playlist_video_url, directory.clone()).await?;

                        ::tokio::try_join!(
                            async {
                                use ::futures_util::StreamExt as _;

                                ::futures_util::pin_mut!(video_event_stream);

                                while let Some(event) = video_event_stream.next().await {
                                    match event {
                                        VideoDownloadEvent::Completed(VideoDownloadCompletedEvent { ref video }) => {
                                            completed.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                            playlist_videos.lock().await.push(video.clone());

                                            let event = PlaylistDownloadProgressUpdatedEvent {
                                                video: video.clone(),

                                                completed: completed.load(::std::sync::atomic::Ordering::Relaxed),
                                                total,
                                            };

                                            playlist_event_stream_tx.send(PlaylistDownloadEvent::ProgressUpdated(event))?;
                                        },
                                        _ => {},
                                    }

                                    video_event_stream_tx.send(event)?;
                                }

                                Ok::<_, ::anyhow::Error>(())
                            },

                            async {
                                use ::futures_util::StreamExt as _;
                            
                                ::futures_util::pin_mut!(video_diagnostic_event_stream);

                                while let Some(event) = video_diagnostic_event_stream.next().await {
                                    diagnostic_event_stream_tx.send(event)?;
                                }

                                Ok::<_, ::anyhow::Error>(())
                            },
                        )?;
                    }

                    Ok::<_, ::anyhow::Error>(())
                }
            });
        }

        let playlist = Playlist {
            id: playlist.id,
            metadata: playlist.metadata,
            videos: ::std::mem::take(&mut *playlist_videos.lock().await),
        };

        let event = PlaylistDownloadCompletedEvent { playlist };
        playlist_event_stream_tx.send(PlaylistDownloadEvent::Completed(event))?;

        let playlist_event_stream = ::async_stream::stream! {
            while let Some(event) = playlist_event_stream_rx.recv().await {
                yield event;
            }
        };

        let video_event_streams = video_event_stream_rxs
            .into_iter()
            .map(|mut video_event_stream_rx| ::async_stream::stream! {
                while let Some(event) = video_event_stream_rx.recv().await {
                    yield event;
                }
            })
            .map(|stream| ::std::boxed::Box::pin(stream) as BoxedStream<VideoDownloadEvent>)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let diagnostic_event_stream = ::async_stream::stream! {
            while let Some(event) = diagnostic_event_stream_rx.recv().await {
                yield event;
            }
        };

        Ok((
            ::std::boxed::Box::pin(playlist_event_stream),
            video_event_streams,
            ::std::boxed::Box::pin(diagnostic_event_stream),
        ))
    }
}

#[derive(new)]
pub struct GenericMetadataWriter;

#[async_trait]
impl MetadataWriter for GenericMetadataWriter {
    async fn write_video(&self, video: &Video) -> Fallible<()> {
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
    async fn write_video(&self, video: &Video) -> Fallible<()> {
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

mod private {
    use domain::{PlaylistMetadata, UnresolvedPlaylist};
    use use_cases::models::PlaylistDownloadStartedEvent;

    use super::*;

    use crate::DownloadVideoView;

    #[async_trait]
    impl Update<VideoDownloadStartedEvent> for DownloadVideoView {
        async fn update(&self, event: &VideoDownloadStartedEvent) -> Fallible<()> {
            use ::colored::Colorize as _;

            self.video_progress_bar
                .println(format!("Downloading video: {}", event.video.metadata.title.white().bold()));

            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadProgressUpdatedEvent> for DownloadVideoView {
        async fn update(&self, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
            self.video_progress_bar.set_position(event.percentage as u64);
            self.video_progress_bar
                .set_prefix(format!("{:>10} @ {:>10} {:>4}", event.size, event.speed, event.eta));
            self.video_progress_bar
                .set_message(format!("{}%", event.percentage));

            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadCompletedEvent> for DownloadVideoView {
        async fn update(&self, _: &VideoDownloadCompletedEvent) -> Fallible<()> {
            static VIDEO_PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = progress_style!("{prefix} {bar:50.green} {msg}");
            
            self.video_progress_bar.set_style(VIDEO_PROGRESS_BAR_FINISH_STYLE.clone());
            self.video_progress_bar.finish();

            Ok(())
        }
    }

    #[async_trait]
    impl Update<PlaylistDownloadStartedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &PlaylistDownloadStartedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Update<PlaylistDownloadProgressUpdatedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &PlaylistDownloadProgressUpdatedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Update<PlaylistDownloadCompletedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadStartedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &VideoDownloadStartedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadProgressUpdatedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadCompletedEvent> for DownloadPlaylistView {
        async fn update(&self, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
            Ok(())
        }
    }

    pub trait FromYtDlpVideoDownloadOutput: Sized {
        fn from_line(line: &str) -> Option<Self>;
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadEvent {
        fn from_line(line: &str) -> Option<Self> {
            VideoDownloadProgressUpdatedEvent::from_line(line).map(Self::ProgressUpdated)
                .or(VideoDownloadStartedEvent::from_line(line).map(Self::Started))
                .or(VideoDownloadCompletedEvent::from_line(line).map(Self::Completed))
        }
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadStartedEvent {
        fn from_line(line: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-started\](?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
            );

            let captures = REGEX.captures(line)?;

            let id = parse_attr(&captures["id"])?;
            let title = parse_attr(&captures["title"])?;
            let album = parse_attr(&captures["album"])?;
            let artists = parse_multivalued_attr(&captures["artist"]);
            let genres = parse_multivalued_attr(&captures["genre"]);

            let video = UnresolvedVideo {
                id,
                metadata: VideoMetadata { title, album, artists, genres },
            };

            Some(Self { video })
        }
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadProgressUpdatedEvent {
        fn from_line(line: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
            );

            let captures = REGEX.captures(line)?;

            let percentage = parse_attr(&captures["percent"])?;
            let eta = parse_attr(&captures["eta"]).unwrap_or("00:00".into());
            let size = parse_attr(&captures["size"])?;
            let speed = parse_attr(&captures["speed"])?;

            Some(Self {
                percentage: percentage.parse().ok()?,
                eta,
                size,
                speed,
            })
        }
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadCompletedEvent {
        fn from_line(line: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
            );

            let captures = REGEX.captures(line)?;

            let id = parse_attr(&captures["id"])?;
            let title = parse_attr(&captures["title"])?;
            let album = parse_attr(&captures["album"])?;
            let artists = parse_multivalued_attr(&captures["artist"]);
            let genres = parse_multivalued_attr(&captures["genre"]);
            let path = parse_attr(&captures["filepath"])?;

            let video = Video {
                id,
                metadata: VideoMetadata { title, album, artists, genres },
                path: ::std::path::PathBuf::from(&*path).into(),
            };

            Some(Self { video })
        }
    }

    impl FromYtDlpVideoDownloadOutput for DownloadDiagnosticEvent {
        fn from_line(line: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"^(?P<level>WARNING|ERROR):\s*(?P<message>.+)$"
            );

            let captures = REGEX.captures(line)?;
            
            let message = parse_attr(&captures["message"])?;
            let level = match &captures["level"] {
                "WARNING" => DiagnosticLevel::Warning,
                "ERROR" => DiagnosticLevel::Error,
                _ => return None,
            };

            Some(Self { level, message })
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

    pub trait FromYtDlpPlaylistDownloadOutput {
        fn from_lines<I, S>(lines: I) -> Option<Self>
        where
            I: IntoIterator<Item = S>,
            S: AsRef<str>,
            Self: Sized;
    }

    impl FromYtDlpPlaylistDownloadOutput for PlaylistDownloadStartedEvent {
        fn from_lines<I, S>(lines: I) -> Option<Self>
        where
            I: IntoIterator<Item = S>,
            S: AsRef<str>,
            Self: Sized,
        {
            static URL_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[playlist-started:url\];(?P<url>[^;]+)"
            );

            static METADATA_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[playlist-started:metadata\];(?P<id>[^;]+);(?P<title>[^;]+)"
            );

            let mut video_urls = Vec::new();

            for line in lines {
                if let Some(captures) = URL_REGEX.captures(line.as_ref()) {
                    let video_url = parse_attr(&captures["url"])?;
                    video_urls.push(video_url);

                } else if let Some(captures) = METADATA_REGEX.captures(line.as_ref()) {
                    let id = parse_attr(&captures["id"])?;
                    let title = parse_attr(&captures["title"])?;

                    let playlist = UnresolvedPlaylist {
                        id,
                        metadata: PlaylistMetadata { title, video_urls },
                    };

                    return Some(Self { playlist })
                }
            }

            None
        }
    }
}
