pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::UnresolvedVideo;
use ::domain::Video;
use ::domain::VideoMetadata;
use use_cases::boundaries::Update;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use use_cases::models::DiagnosticLevel;
use use_cases::models::DownloadDiagnosticEvent;
use ::use_cases::models::PlaylistDownloadEvent;
use ::use_cases::models::VideoDownloadCompletedEvent;
use ::use_cases::models::VideoDownloadProgressUpdatedEvent;
use ::use_cases::models::VideoDownloadEvent;
use ::use_cases::models::VideoDownloadStartedEvent;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;

pub struct DownloadVideoView {
    progress_bar: ::indicatif::ProgressBar,
}

// TODO migrate to with_key
impl DownloadVideoView {
    pub fn new() -> Fallible<Self> {
        let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")?;
        let progress_bar = ::indicatif::ProgressBar::new(100).with_style(progress_bar_style);

        progress_bar.set_prefix(format!("{:>10} {:>10} {:>4}", "??MiB", "??MiB/s", "??:??"));
        progress_bar.set_message("??%");

        Ok(Self { progress_bar })
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

        match event.level {
            DiagnosticLevel::Warning => {
                self.progress_bar.println(format!("{}", event.message.yellow().bold()));
            },

            DiagnosticLevel::Error => {
                let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.red} {msg}")?;
                self.progress_bar.set_style(progress_bar_style);

                self.progress_bar.abandon();
                eprintln!("{}", event.message.red().bold());
            },
        }
        
        Ok(())
    }
}

// pub struct DownloadPlaylistView;

// #[async_trait]
// impl DownloadVideoOutputBoundary for DownloadPlaylistView {
//     async fn update(&self, _event: &VideoDownloadEvent) -> Fallible<()> { todo!() }
// }

// #[async_trait]
// impl DownloadPlaylistOutputBoundary for DownloadPlaylistView {
//     async fn update(&self, _event: &PlaylistDownloadEvent) -> Fallible<()> { todo!() }
// }

#[derive(new)]
pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DownloadDiagnosticEvent>)> {
        use ::std::io::BufRead as _;

        #[rustfmt::skip]
        let mut process = ::std::process::Command::new("yt-dlp")
            .args([
                &*url,
                "--paths", &directory.to_str().unwrap(),
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
                "--print", "post_process:[video-completed]%(filepath)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            ])
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();

        let stdout_reader = ::std::io::BufReader::new(stdout);
        let stderr_reader = ::std::io::BufReader::new(stderr);

        let video_events = ::async_stream::stream! {
            use crate::private::FromYtDlpVideoDownloadOutput as _;

            for event in stdout_reader.lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| VideoDownloadEvent::from_str(&line))
            {
                yield event;
            }
        };

        let diagnostic_events = ::async_stream::stream! {
            use crate::private::FromYtDlpVideoDownloadOutput as _;

            for event in stderr_reader.lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| DownloadDiagnosticEvent::from_str(&line))
            {
                yield event;
            }
        };

        Ok((
            ::std::boxed::Box::pin(video_events),
            ::std::boxed::Box::pin(diagnostic_events),
        ))
    }

    async fn download_playlist(
        &self, _url: MaybeOwnedString, _directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>, BoxedStream<DownloadDiagnosticEvent>)> {
        // use ::std::io::BufRead as _;

        // #[rustfmt::skip]
        // let command = ::duct::cmd!(
        //     "yt-dlp",
        //     &*url,
        //     "--paths", &*directory,
        //     "--quiet",
        //     "--flat-playlist",
        //     "--color", "no_color",
        //     "--print", "playlist:[playlist-started]%(id)s;%(title)s",
        //     "--print", "video:[playlist-started]%(url)s"
        // );

        todo!()
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
    use super::*;

    use crate::DownloadVideoView;

    #[async_trait]
    impl Update<VideoDownloadStartedEvent> for DownloadVideoView {
        async fn update(&self, event: &VideoDownloadStartedEvent) -> Fallible<()> {
            use ::colored::Colorize as _;

            self.progress_bar
                .println(format!("downloading `{}` ...", event.video.metadata.title.white().bold()));

            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadProgressUpdatedEvent> for DownloadVideoView {
        async fn update(&self, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
            self.progress_bar.set_position(event.percentage as u64);
            self.progress_bar
                .set_prefix(format!("{:>10} {:>10} {:>4}", event.size, event.speed, event.eta));
            self.progress_bar.set_message(format!("{}%", event.percentage));

            Ok(())
        }
    }

    #[async_trait]
    impl Update<VideoDownloadCompletedEvent> for DownloadVideoView {
        async fn update(&self, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
            use ::colored::Colorize as _;

            let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.green} {msg}")?;
            self.progress_bar.set_style(progress_bar_style);

            self.progress_bar.finish();
            println!(" downloaded `{}`.", event.video.metadata.title.green().bold());

            Ok(())
        }
    }

    pub trait FromYtDlpVideoDownloadOutput: Sized {
        fn from_str(string: &str) -> Option<Self>;
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadEvent {
        fn from_str(string: &str) -> Option<Self> {
            VideoDownloadProgressUpdatedEvent::from_str(string).map(Self::ProgressUpdated)
                .or(VideoDownloadStartedEvent::from_str(string).map(Self::Started))
                .or(VideoDownloadCompletedEvent::from_str(string).map(Self::Completed))
        }
    }

    impl FromYtDlpVideoDownloadOutput for VideoDownloadStartedEvent {
        fn from_str(string: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-started\](?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
            );

            let captures = REGEX.captures(string)?;

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
        fn from_str(string: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
            );

            let captures = REGEX.captures(string)?;

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
        fn from_str(string: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
            );

            let captures = REGEX.captures(string)?;

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
        fn from_str(string: &str) -> Option<Self> {
            static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
                r"^(?P<level>WARNING|ERROR):\s*(?P<message>.+)$"
            );

            let captures = REGEX.captures(string)?;
            
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
}
