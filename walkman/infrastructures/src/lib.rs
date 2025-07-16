pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::UnresolvedVideo;
use ::domain::Video;
use ::domain::VideoMetadata;
use ::use_cases::boundaries::DownloadPlaylistOutputBoundary;
use ::use_cases::boundaries::DownloadVideoOutputBoundary;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use ::use_cases::models::PlaylistDownloadEvent;
use ::use_cases::models::VideoDownloadCompletedEvent;
use ::use_cases::models::VideoDownloadProgressUpdatedEvent;
use ::use_cases::models::VideoDownloadEvent;
use ::use_cases::models::VideoFailedEvent;
use ::use_cases::models::VideoDownloadStartedEvent;
use ::use_cases::models::VideoWarningEvent;

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
impl DownloadVideoOutputBoundary for DownloadVideoView {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        match event {
            VideoDownloadEvent::Started(event) => self.update_on_video_started_event(event),
            VideoDownloadEvent::ProgressUpdated(event) => self.update_on_video_downloading_event(event),
            VideoDownloadEvent::Completed(event) => self.update_on_video_completed_video_event(event),
            VideoDownloadEvent::Warning(event) => self.update_on_video_warning_event(event),
            VideoDownloadEvent::Failed(event) => self.update_on_video_failed_event(event),
        }
    }
}

impl DownloadVideoView {
    fn update_on_video_started_event(&self, event: &VideoDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        self.progress_bar
            .println(format!("downloading `{}` ...", event.video.metadata.title.white().bold()));

        Ok(())
    }

    fn update_on_video_downloading_event(&self, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        self.progress_bar.set_position(event.percentage as u64);
        self.progress_bar
            .set_prefix(format!("{:>10} {:>10} {:>4}", event.size, event.speed, event.eta));
        self.progress_bar.set_message(format!("{}%", event.percentage));

        Ok(())
    }

    fn update_on_video_completed_video_event(&self, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.green} {msg}")?;
        self.progress_bar.set_style(progress_bar_style);

        self.progress_bar.finish();
        println!(" downloaded `{}`.", event.video.metadata.title.green().bold());

        Ok(())
    }

    fn update_on_video_warning_event(&self, event: &VideoWarningEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        self.progress_bar.println(format!("{}", event.message.yellow().bold()));

        Ok(())
    }

    fn update_on_video_failed_event(&self, event: &VideoFailedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.red} {msg}")?;
        self.progress_bar.set_style(progress_bar_style);

        self.progress_bar.abandon();
        eprintln!("{}", event.message.red().bold());

        Ok(())
    }
}

pub struct DownloadPlaylistView;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &VideoDownloadEvent) -> Fallible<()> { todo!() }
}

#[async_trait]
impl DownloadPlaylistOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &PlaylistDownloadEvent) -> Fallible<()> { todo!() }
}

#[derive(new)]
pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<BoxedStream<VideoDownloadEvent>> {
        use ::std::io::BufRead as _;

        #[rustfmt::skip]
        let command = ::duct::cmd!(
            "yt-dlp",
            &*url,
            "--paths", &*directory,
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
        );

        let reader_handle = command.stderr_to_stdout().reader()?;
        let reader = ::std::io::BufReader::new(reader_handle);

        let events = ::async_stream::stream! {
            for event in reader.lines()
                .filter_map(|line| line.ok())
                .filter_map(|line| Self::parse_video_event(&line))
            {
                yield event;
            }
        };

        Ok(::std::boxed::Box::pin(events))
    }

    async fn download_playlist(
        &self, url: MaybeOwnedString, directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>)> {
        use ::std::io::BufRead as _;

        #[rustfmt::skip]
        let command = ::duct::cmd!(
            "yt-dlp",
            &*url,
            "--paths", &*directory,
            "--quiet",
            "--flat-playlist",
            "--color", "no_color",
            "--print", "playlist:[playlist-started]%(id)s;%(title)s",
            "--print", "video:[playlist-started]%(url)s"
        );

        
    }
}

impl YtDlpDownloader {
    fn parse_video_event(line: &str) -> Option<VideoDownloadEvent> {
        Self::parse_video_downloading_event(line)
            .map(VideoDownloadEvent::ProgressUpdated)
            .or_else(|| Self::parse_video_started_event(line).map(VideoDownloadEvent::Started))
            .or_else(|| Self::parse_video_completed_event(line).map(VideoDownloadEvent::Completed))
            .or_else(|| Self::parse_video_warning_event(line).map(VideoDownloadEvent::Warning))
            .or_else(|| Self::parse_video_failed_event(line).map(VideoDownloadEvent::Failed))
            .or_else(|| None)
    }

    fn parse_video_started_event(line: &str) -> Option<VideoDownloadStartedEvent> {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-started\](?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        );

        let captures = REGEX.captures(line)?;

        let id = Self::parse_attr(&captures["id"])?;
        let title = Self::parse_attr(&captures["title"])?;
        let album = Self::parse_attr(&captures["album"])?;
        let artists = Self::parse_multivalued_attr(&captures["artist"]);
        let genres = Self::parse_multivalued_attr(&captures["genre"]);

        let video = UnresolvedVideo {
            id,
            metadata: VideoMetadata { title, album, artists, genres },
        };

        Some(VideoDownloadStartedEvent { video })
    }

    fn parse_video_downloading_event(line: &str) -> Option<VideoDownloadProgressUpdatedEvent> {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
        );

        let captures = REGEX.captures(line)?;

        let percentage = Self::parse_attr(&captures["percent"])?;
        let eta = Self::parse_attr(&captures["eta"]).unwrap_or("00:00".into());
        let size = Self::parse_attr(&captures["size"])?;
        let speed = Self::parse_attr(&captures["speed"])?;

        Some(VideoDownloadProgressUpdatedEvent {
            percentage: percentage.parse().ok()?,
            eta,
            size,
            speed,
        })
    }

    fn parse_video_completed_event(line: &str) -> Option<VideoDownloadCompletedEvent> {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        );

        let captures = REGEX.captures(line)?;

        let id = Self::parse_attr(&captures["id"])?;
        let title = Self::parse_attr(&captures["title"])?;
        let album = Self::parse_attr(&captures["album"])?;
        let artists = Self::parse_multivalued_attr(&captures["artist"]);
        let genres = Self::parse_multivalued_attr(&captures["genre"]);
        let path = Self::parse_attr(&captures["filepath"])?;

        let video = Video {
            id,
            metadata: VideoMetadata { title, album, artists, genres },
            path: ::std::path::PathBuf::from(&*path).into(),
        };

        Some(VideoDownloadCompletedEvent { video })
    }

    fn parse_video_warning_event(line: &str) -> Option<VideoWarningEvent> {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(r"^WARNING:\s*(?P<message>.+)$");

        let captures = REGEX.captures(line)?;
        let message = Self::parse_attr(&captures["message"])?;

        Some(VideoWarningEvent { message })
    }

    fn parse_video_failed_event(line: &str) -> Option<VideoFailedEvent> {
        static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(r"^ERROR:\s*(?P<message>.+)$");

        let captures = REGEX.captures(line)?;
        let message = Self::parse_attr(&captures["message"])?;

        Some(VideoFailedEvent { message })
    }

    fn parse_multivalued_attr(captured: &str) -> Vec<MaybeOwnedString> {
        match Self::parse_attr(captured) {
            Some(attrs) => attrs
                .split(',')
                .map(Self::normalize)
                .map(|attr| attr.to_owned().into())
                .collect(),
            None => Vec::new(),
        }
    }

    fn parse_attr(captured: &str) -> Option<MaybeOwnedString> {
        let captured = Self::normalize(captured);

        if captured == "NA" {
            None
        } else {
            Some(captured.to_owned().into())
        }
    }

    fn normalize(captured: &str) -> &str { captured.trim() }

    fn parse_
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
