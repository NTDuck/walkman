mod utils;

use std::path::Path;

use infrastructures::{DownloadVideoView, LoftyMetadataWriter, YtDlpDownloader};
use triomphe::Arc;

use crate::utils::aliases::MaybeOwnedPath;

#[tokio::main]
async fn main() {
    let download_video_view = Arc::new(DownloadVideoView::new());
    let downloader = Arc::new(YtDlpDownloader::new());
    let metadata_writer = Arc::new(LoftyMetadataWriter::new());

    // let 
}