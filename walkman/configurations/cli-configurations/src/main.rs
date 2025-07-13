mod utils;

use std::{path::PathBuf, sync::Arc};

use clap::{value_parser, Arg, Command};
use infrastructures::{DownloadVideoView, LoftyMetadataWriter, YtDlpDownloader};
use use_cases::{boundaries::{DownloadVideoInputBoundary, DownloadVideoRequestModel}, interactors::DownloadVideoInteractor};

use crate::utils::aliases::{MaybeOwnedPath, MaybeOwnedString};

#[tokio::main]
#[allow(unused_variables)]
async fn main() {
    let download_video_view = Arc::new(DownloadVideoView::new());
    let downloader = Arc::new(YtDlpDownloader::new());
    let metadata_writer = Arc::new(LoftyMetadataWriter::new());

    let download_video_interactor = DownloadVideoInteractor::new(
        download_video_view.clone(),
        downloader.clone(),
        metadata_writer.clone(),
    );

    let command = Command::new("walkman")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("download-video")
            .arg(Arg::new("url")
                .short('i')
                .default_value("https://youtu.be/dQw4w9WgXcQ?list=RDdQw4w9WgXcQ")
                .value_parser(value_parser!(String)))
            .arg(Arg::new("directory")
                .short('o')
                .default_value(env!("CARGO_WORKSPACE_DIR"))
                .value_parser(value_parser!(PathBuf))));

    match command.get_matches().subcommand() {
        Some(("download-video", matches)) => {
            let url = matches
                .get_one::<String>("url")
                .expect("Error: Missing required argument `url`");

            let directory = matches
                .get_one::<PathBuf>("directory")
                .expect("Error: Missing required argument `directory`");

            let model = DownloadVideoRequestModel {
                url: MaybeOwnedString::Owned(url.to_string()),
                directory: MaybeOwnedPath::Owned(directory.to_path_buf()),
            };

            download_video_interactor.apply(model).await;
        },
        _ => unreachable!(),
    }
}
