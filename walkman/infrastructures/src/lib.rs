pub(crate) mod utils;

use ::async_trait::async_trait;
use ::derive_new::new;
use ::domain::Video;
use ::domain::VideoMetadata;
use ::use_cases::boundaries::DownloadPlaylistOutputBoundary;
use ::use_cases::boundaries::DownloadVideoOutputBoundary;
use ::use_cases::gateways::Downloader;
use ::use_cases::gateways::MetadataWriter;
use ::use_cases::gateways::PlaylistDownloadEvent;
use ::use_cases::gateways::VideoDownloadEvent;

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
        let progress_bar = ::indicatif::ProgressBar::new(100)
            .with_style(progress_bar_style);

        progress_bar.set_prefix(format!("{:>10} {:>10} {:>4}", "??MiB", "??MiB/s", "??:??"));
        progress_bar.set_message("??%");

        Ok(Self {
            progress_bar,
        })
    }
}

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadVideoView {
    async fn update(&self, event: &VideoDownloadEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        match event {
            VideoDownloadEvent::Downloading {
                percentage,
                eta,
                size,
                speed,
            } => {
                self.progress_bar
                    .set_position(*percentage as u64);
                self.progress_bar
                    .set_prefix(format!("{:>10} {:>10} {:>4}", size, speed, eta));
                self.progress_bar
                    .set_message(format!("{}%", percentage));
            },

            VideoDownloadEvent::Completed(video) => {
                let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.green} {msg}")?;
                self.progress_bar.set_style(progress_bar_style);

                self.progress_bar.finish();
                println!("Downloaded {}.", video.metadata.title.green().bold());
            },

            VideoDownloadEvent::Failed(error) => {
                let progress_bar_style = ::indicatif::ProgressStyle::with_template("{prefix} {bar:50.red} {msg}")?;
                self.progress_bar.set_style(progress_bar_style);

                self.progress_bar.abandon();
                eprintln!("{}", error.red().bold());
            },
        }

        Ok(())
    }
}

pub struct DownloadPlaylistView;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &VideoDownloadEvent) -> Fallible<()> {
        todo!()
    }
}

#[async_trait]
impl DownloadPlaylistOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &PlaylistDownloadEvent) -> Fallible<()> {
        todo!()
    }
}

#[derive(new)]
pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(
        &self,
        url: MaybeOwnedString,
        directory: MaybeOwnedPath,
    ) -> Fallible<BoxedStream<VideoDownloadEvent>> {
        use ::std::io::BufRead as _;

        let command = ::duct::cmd!(
            "yt-dlp",
            &*url,
            "--paths",
            &*directory,
            "--format",
            "bestaudio",
            "--extract-audio",
            "--audio-format",
            "mp3",
            "--output",
            "%(title)s.%(ext)s",
            "--quiet",
            "--newline",
            "--abort-on-error",
            "--no-playlist",
            "--force-overwrites",
            "--progress",
            "--progress-template",
            "[video-downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%\
             (progress._speed_str)s",
            "--exec",
            "echo [video-completed]%(filepath)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            "--color",
            "no_color",
        );

        let reader_handle = command.stderr_to_stdout().reader()?;
        let reader = ::std::io::BufReader::new(reader_handle);

        static DOWNLOADING_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
        );
        static COMPLETED_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(
            r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        );
        static FAILED_REGEX: ::once_cell::sync::Lazy<::regex::Regex> = regex!(r"^ERROR:\s*(?P<error>.+)$");

        let events = ::async_stream::stream! {
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => break,
                };

                if let Some(captures) = DOWNLOADING_REGEX.captures(&line) {
                    if let (Some(percentage), Some(size), Some(speed)) = (
                        Self::parse_attr(&captures["percent"]),
                        Self::parse_attr(&captures["size"]),
                        Self::parse_attr(&captures["speed"]),
                    ) {
                        let eta = Self::parse_attr(&captures["eta"]);

                        yield VideoDownloadEvent::Downloading {
                            percentage: percentage.parse().unwrap_or(0),
                            eta: eta.unwrap_or("00:00".into()),
                            size,
                            speed,
                        };
                    }

                } else if let Some(captures) = COMPLETED_REGEX.captures(&line) {
                    if let (Some(id), Some(title), Some(album), Some(path)) = (
                        Self::parse_attr(&captures["id"]),
                        Self::parse_attr(&captures["title"]),
                        Self::parse_attr(&captures["album"]),
                        Self::parse_attr(&captures["filepath"]),
                    ) {
                        let artists = Self::parse_multivalued_attr(&captures["artist"]);
                        let genres = Self::parse_multivalued_attr(&captures["genre"]);

                        yield VideoDownloadEvent::Completed(Video {
                            id,
                            metadata: VideoMetadata {
                                title,
                                album,
                                artists,
                                genres,
                            },
                            path: ::std::path::PathBuf::from(&*path).into(),
                        });
                    }

                } else if let Some(captures) = FAILED_REGEX.captures(&line) {
                    if let Some(error) = Self::parse_attr(&captures["error"]) {
                        yield VideoDownloadEvent::Failed(error);
                    }
                }
            }
        };

        Ok(::std::boxed::Box::pin(events))
    }

    async fn download_playlist(
        &self,
        _url: MaybeOwnedString,
        _directory: MaybeOwnedPath,
    ) -> Fallible<(BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>)> {
        todo!()
    }
}

impl YtDlpDownloader {
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

    fn normalize(captured: &str) -> &str {
        captured.trim()
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

        let mut file = ::lofty::read_from_path(video.path.clone())?;

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

        let metadata = video.metadata.clone();

        tag.set_title(metadata.title.into_owned());
        tag.set_album(metadata.album.into_owned());
        tag.set_artist(metadata.artists.join(", "));
        tag.set_genre(metadata.genres.join(", "));

        tag.save_to_path(video.path.clone(), ::lofty::config::WriteOptions::default().respect_read_only(false))?;

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

        let metadata = video.metadata.clone();

        tag.set_title(metadata.title);
        tag.set_album(metadata.album);
        tag.set_artist(metadata.artists.join(", "));
        tag.set_genre(metadata.genres.join(", "));

        tag.write_to_path(video.path.clone(), ::id3::Version::Id3v23)?;

        Ok(())
    }
}
