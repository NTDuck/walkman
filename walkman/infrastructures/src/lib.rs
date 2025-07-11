use async_stream::stream;
use async_trait::async_trait;
use domain::{Playlist, Video};
use use_cases::gateways::{DownloadError, Downloader, PlaylistDownloadSnapshot, Stream, VideoDownloadSnapshot};

pub struct YtDlpDownloader;

#[async_trait]
impl Downloader for YtDlpDownloader {
    async fn download_video(&self, _url: String) -> Result<(Video, Stream<VideoDownloadSnapshot>), DownloadError> {
        Ok((
            Video::default(),
            Box::pin(stream! {
                for _ in 0..10 {
                    yield VideoDownloadSnapshot::default();
                }
            }),
        ))
    }

    async fn download_playlist(&self, _url: String) -> Result<(Playlist, Stream<PlaylistDownloadSnapshot>, Stream<VideoDownloadSnapshot>), DownloadError> {
        Ok((
            Playlist::default(),
            Box::pin(stream! {
                for _ in 0..10 {
                    yield PlaylistDownloadSnapshot::default();
                }
            }),
            Box::pin(stream! {
                for _ in 0..10 {
                    yield VideoDownloadSnapshot::default();
                }
            }),
        ))
    }
}
