mod utils;

use std::{io::{BufRead, BufReader}, path::PathBuf, process::{Command, ExitCode, Stdio}};

use async_stream::stream;
use async_trait::async_trait;
use domain::{Video, VideoMetadata};
use once_cell::sync::Lazy;
use regex::Regex;
use use_cases::{boundaries::{DownloadPlaylistOutputBoundary, DownloadVideoOutputBoundary}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

use crate::utils::aliases::{BoxedStream, MaybeOwnedPath, MaybeOwnedString};

pub struct DownloadVideoProgressBar;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadVideoProgressBar {
    async fn update(&self, _event: &VideoDownloadEvent) {

    }
}

pub struct DownloadPlaylistAndVideoProgressBar;

#[async_trait]
impl DownloadVideoOutputBoundary for DownloadPlaylistAndVideoProgressBar {
    async fn update(&self, _event: &VideoDownloadEvent) {
        
    }
}

#[async_trait]
impl DownloadPlaylistOutputBoundary for DownloadPlaylistAndVideoProgressBar {
    async fn update(&self, _event: &PlaylistDownloadEvent) {

    }
}

pub struct YtDlpDownloader {
    configurations: YtDlpDownloaderConfigurations,
}

pub struct YtDlpDownloaderConfigurations {
    directory: MaybeOwnedPath,
}

impl YtDlpDownloader {
    pub fn new(configurations: YtDlpDownloaderConfigurations) -> Self {
        Self {
            configurations,
        }
    }
}

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(&self, url: MaybeOwnedString) -> BoxedStream<VideoDownloadEvent> {
        use VideoDownloadEvent::*;

        let mut command = Command::new("yt-dlp");
        command
            .args([
                &*url,
                "--format", "bestaudio",
                "--output", "\"%(title)s.%(ext)s\"",
                "--paths", &self.configurations.directory.to_string_lossy(),
                "--quiet",
                "--newline",
                "--no-playlist",
                "--no-abort-on-error",
                "--no-plugin-dirs",
                "--progress",
                "--progress-template", "\"[downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s\"",
                "--exec", "\"echo [completed]%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s\"",
                // "--flat-playlist",
                "--color", "no_color",
                // "--min-filesize",
                "--max-filesize", "44.6M",
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
            r"\[downloading\](?P<percent>\d+)\.\d+%;(?P<eta>[^;]+);(?P<size>[^;]+);(?P<speed>[^\r\n]+)"
        ).unwrap());

        static COMPLETED_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"\[completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
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

    async fn download_playlist(&self, _url: MaybeOwnedString) -> (BoxedStream<PlaylistDownloadEvent>, BoxedStream<VideoDownloadEvent>) {
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
            vec![]
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

#[async_trait]
impl MetadataWriter for LoftyMetadataWriter {
    async fn write_video(&self, _video: &Video) {
        
    }
}
