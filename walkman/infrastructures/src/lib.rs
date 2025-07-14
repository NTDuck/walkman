pub(crate) mod utils;

use std::{io::{BufRead, BufReader}, path::PathBuf};

use async_stream::stream;
use async_trait::async_trait;
use derive_new::new;
use domain::{Video, VideoMetadata};
use once_cell::sync::Lazy;
use regex::Regex;
use use_cases::{boundaries::{DownloadPlaylistOutputBoundary, DownloadVideoOutputBoundary}, gateways::{Downloader, MetadataWriter, PlaylistDownloadEvent, VideoDownloadEvent}};

use crate::utils::aliases::{BoxedStream, MaybeOwnedPath, MaybeOwnedString};

pub struct DownloadVideoView {
    progress_bar: indicatif::ProgressBar,
}

impl DownloadVideoView {
    pub fn new() -> Self {
        Self {
            progress_bar: indicatif::ProgressBar::new(100)
                .with_style(indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")
                    .expect("Error: Invalid progress bar template string")),
        }
    }
}

impl Default for DownloadVideoView {
    fn default() -> Self {
        Self::new()
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
                self.progress_bar.set_prefix(format!("{:>10} {:>10} {:>4}", size, speed, eta));
                self.progress_bar.set_message(format!("{}%", percentage));
            },
            Completed(video) => {
                self.progress_bar.finish();
                println!("Downloaded `{}`", video.metadata.title);
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

#[derive(new)]
pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(&self, url: MaybeOwnedString, directory: MaybeOwnedPath) -> BoxedStream<VideoDownloadEvent> {
        use VideoDownloadEvent::*;

        let command = duct::cmd!(
            "yt-dlp",
            &*url,
            "--paths", &*directory,
            "--format", "bestaudio",
            "--extract-audio",
            "--audio-format", "mp3",
            "--output", "%(title)s.%(ext)s",
            "--quiet",
            "--newline",
            "--no-playlist",
            "--force-overwrites",
            "--progress",
            "--progress-template", "[video-downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s",
            "--exec", "echo [video-completed]%(filepath)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s",
            "--color", "no_color",
        );

        let reader_handle = command.stderr_to_stdout().reader()
            .expect("Error: Failed to read stdout");
        let reader = BufReader::new(reader_handle);

        static DOWNLOADING_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"\[video-downloading\]\s*(?P<percent>\d+)(?:\.\d+)?%;(?P<eta>[^;]+);\s*(?P<size>[^;]+);\s*(?P<speed>[^\r\n]+)"
        ).expect("Error: Invalid regex string"));

        static COMPLETED_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(
            r"\[video-completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
        ).expect("Error: Invalid regex string"));

        Box::pin(stream! {
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        yield Failed(format!("Error: Failed to read line with error `{}`", error).into());
                        continue;
                    }
                };

                if let Some(captures) = DOWNLOADING_REGEX.captures(&line) {
                    yield Downloading {
                        percentage: Self::parse_attr(&captures["percent"])
                            .expect(&format!("Error: Failed to regex-capture `percent` from line `{}`", line))
                            .parse()
                            .expect(&format!("Error: Failed to parse `u8` from regex-captured string")),
                        eta: Self::parse_attr(&captures["eta"])
                            .unwrap_or("00:00".into()),
                        size: Self::parse_attr(&captures["size"])
                            .expect(&format!("Error: Failed to regex-capture `size` from line `{}`", line)),
                        speed: Self::parse_attr(&captures["speed"])
                            .expect(&format!("Error: Failed to regex-capture `speed` from line `{}`", line)),
                    };
                } else if let Some(captures) = COMPLETED_REGEX.captures(&line) {
                    yield Completed(Video {
                        id: Self::parse_attr(&captures["id"])
                                .expect(&format!("Error: Failed to regex-capture `id` from line `{}`", line)),
                        metadata: VideoMetadata {
                            title: Self::parse_attr(&captures["title"])
                                .expect(&format!("Error: Failed to regex-capture `title` from `{}`", line)),
                            album: Self::parse_attr(&captures["album"])
                                .expect(&format!("Error: Failed to regex-capture `album` from `{}`", line)),
                            artists: Self::parse_multivalued_attr(&captures["artist"]),
                            genres: Self::parse_multivalued_attr(&captures["genre"]),
                        },
                        path: MaybeOwnedPath::Owned(PathBuf::from(
                            &*Self::parse_attr(&captures["filepath"])
                                .expect(&format!("Error: Failed to regex-capture `filepath` from line `{}`", line))
                        )),
                    });
                } else {
                    yield Failed(format!("Error: Failed to regex-capture line `{}`", line).into());
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
    fn parse_attr<'a>(captured: &'a str) -> Option<MaybeOwnedString> {
        let captured = Self::normalize(captured);

        if captured == "NA" {
            None
        } else {
            Some(captured.to_owned().into())
        }
    }

    fn parse_multivalued_attr<'a>(captured: &'a str) -> Vec<MaybeOwnedString> {
        match Self::parse_attr(captured) {
            Some(attrs) => attrs.split(',')
                .map(|attr| Self::normalize(attr))
                .map(|attr| attr.to_owned().into())
                .collect(),
            None => Vec::new(),
        }
    }

    fn normalize<'a>(captured: &'a str) -> &'a str {
        captured.trim()
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

#[derive(new)]
pub struct GenericMetadataWriter;

#[async_trait]
impl MetadataWriter for GenericMetadataWriter {
    async fn write_video(&self, video: &Video) {
        use lofty::prelude::*;

        let mut file = lofty::read_from_path(video.path.clone())
            .expect(&format!("Error: Failed to read file `{:?}`", video.path));

        let tag = match file.primary_tag_mut() {
            Some(tag) => tag,
            None => match file.first_tag_mut() {
                Some(tag) => tag,
                None => {
                    file.insert_tag(lofty::tag::Tag::new(file.primary_tag_type()));
                    file.primary_tag_mut().unwrap()
                },
            },
        };
        
        let metadata = video.metadata.clone();

        tag.set_title(metadata.title.into_owned());
        tag.set_album(metadata.album.into_owned());
        tag.set_artist(metadata.artists.join(", "));
        tag.set_genre(metadata.genres.join(", "));

        tag.save_to_path(video.path.clone(), lofty::config::WriteOptions::default().respect_read_only(false))
            .expect(&format!("Error: Failed to write to file `{:?}`", video.path));
    }
}

#[derive(new)]
pub struct Id3MetadataWriter;

#[async_trait]
impl MetadataWriter for Id3MetadataWriter {
    async fn write_video(&self, video: &Video) {
        use id3::TagLike;

        let mut tag = id3::Tag::new();

        let metadata = video.metadata.clone();

        tag.set_title(metadata.title);
        tag.set_album(metadata.album);
        tag.set_artist(metadata.artists.join(", "));
        tag.set_genre(metadata.genres.join(", "));

        tag.write_to_path(video.path.clone(), id3::Version::Id3v23)
            .expect(&format!("Error: Failed to write to file `{:?}`", video.path));
    }
}
