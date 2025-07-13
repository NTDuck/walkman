mod utils;

use std::{io::{BufRead, BufReader}, path::PathBuf, process::{Command, ExitCode, Stdio}};

use async_stream::stream;
use async_trait::async_trait;
use domain::{Video, VideoMetadata};
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use regex::Regex;
use use_cases::{boundaries::{DownloadPlaylistOutputBoundary, DownloadVideoOutputBoundary}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

use crate::utils::aliases::{BoxedStream, MaybeOwnedPath, MaybeOwnedString};

pub struct DownloadVideoView {
    progress_bar: ProgressBar,
}

impl DownloadVideoView {
    pub fn new() -> Self {
        Self {
            progress_bar: ProgressBar::new(100)
                .with_style(ProgressStyle::with_template("{prefix} [{bar:44}] {msg}")
                    .unwrap()
                    .progress_chars("█░ ")),
        }
    }
}

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadVideoView {
    async fn update(&self, event: &VideoDownloadEvent) {
        use VideoDownloadEvent::*;

        match event {
            Downloading {
                percentage,
                eta,
                size,
                speed,
            } => {
                self.progress_bar.set_position(*percentage as u64);
                self.progress_bar.set_prefix(format!("[{speed}\t{eta}]"));
                self.progress_bar.set_message(format!("[{percentage}% of {size}]"));
            },
            Completed(video) => {
                self.progress_bar.finish_with_message(format!("{}", video.metadata.title));
            },
            Failed(error) => {
                self.progress_bar.abandon_with_message(format!("{:?}", error));
            },
        }
    }
}

pub struct DownloadPlaylistView;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &VideoDownloadEvent) {
        
    }
}

#[async_trait]
impl DownloadPlaylistOutputBoundary for DownloadPlaylistView {
    async fn update(&self, _event: &PlaylistDownloadEvent) {

    }
}

pub struct YtDlpDownloader;

impl YtDlpDownloader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(&self, url: MaybeOwnedString, directory: MaybeOwnedPath) -> BoxedStream<VideoDownloadEvent> {
        use VideoDownloadEvent::*;

        let mut command = Command::new("yt-dlp");
        command
            .args([
                &*url,
                "--paths", &directory.to_string_lossy(),
                "--format", "bestaudio",
                "--audio-format", "mp3",
                "--output", "\"%(title)s.%(ext)s\"",
                "--quiet",
                "--newline",
                "--no-playlist",
                "--progress",
                "--progress-template", "\"[video-downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s\"",
                "--exec", "\"echo [video-completed]%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s\"",
                // "--flat-playlist",
                "--color", "no_color",
            ])
            // Merge stderr into stdout
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let mut process = match command.spawn() {
            Ok(process) => process,
            Err(_) => {
                return Box::pin(stream! {
                    yield Failed(ExitCode::FAILURE);
                });
            },
        };

        let stdout = match process.stdout.take() {
            Some(stdout) => stdout,
            None => {
                return Box::pin(stream! {
                    yield Failed(ExitCode::FAILURE);
                });
            },
        };

        let reader = BufReader::new(stdout);

        static DOWNLOADING_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"\[video-downloading\](?P<percent>\d+)\.\d+%;(?P<eta>[^;]+);(?P<size>[^;]+);(?P<speed>[^\r\n]+)"
        ).unwrap());

        static COMPLETED_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        ).unwrap());

        Box::pin(stream! {
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => {
                        yield Failed(ExitCode::FAILURE);
                        continue;
                    }
                };

                if let Some(captures) = DOWNLOADING_REGEX.captures(&line) {
                    yield Downloading {
                        percentage: captures["percent"].parse().unwrap(),
                        eta: captures["eta"].trim().to_string().into(),
                        size: captures["size"].trim().to_string().into(),
                        speed: captures["speed"].trim().to_string().into(),
                    };
                } else if let Some(captures) = COMPLETED_REGEX.captures(&line) {
                    yield Completed(Video {
                        id: captures["id"].trim().to_string().into(),
                        metadata: VideoMetadata {
                            title: captures["title"].trim().to_string().into(),
                            album: captures["album"].trim().to_string().into(),
                            artists: Self::parse_multivalued_attr(captures["artists"].trim().to_string().into()),
                            genres: Self::parse_multivalued_attr(captures["genres"].trim().to_string().into()),
                        },
                        path: MaybeOwnedPath::Owned(PathBuf::from(captures["filepath"].trim())),
                    });
                } else {
                    yield Failed(ExitCode::FAILURE);
                }
            }
        })
    }

    async fn download_playlist(&self, _url: MaybeOwnedString, _directory: MaybeOwnedPath) -> (BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>) {
        (
            Box::pin(stream! {
                yield PlaylistDownloadEvent::Failed(Default::default());
            }),
            Box::pin(stream! {
                yield VideoDownloadEvent::Failed(Default::default());
            }),
        )
    }
}

impl YtDlpDownloader {
    fn parse_multivalued_attr(attrs: MaybeOwnedString) -> Vec<MaybeOwnedString> {
        if attrs == "NA" {
            Vec::new()
        } else {
            attrs.split(",")
                .map(|attr| attr.trim().to_string())
                .filter(|attr| !attr.is_empty())
                .map(Into::into)
                .collect()
        }
    }
}


/*
Options:
--no-abort-on-error
--no-plugin-dirs
--flat-playlist
--color no_color
--min-filesize ???
--max-filesize 44.6M

Video only:
--no-playlist

Playlist only:
--yes-playlist

Update:
--download-archive [xxx] (will be a file in the current dir)
--no-break-on-existing


Initial check-log-stuff:
--dump-user-agent: 

Consider:
- skip livestreams.

*/

pub struct LoftyMetadataWriter;

impl LoftyMetadataWriter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MetadataWriter for LoftyMetadataWriter {
    async fn write_video(&self, _video: &Video) {
        
    }
}
