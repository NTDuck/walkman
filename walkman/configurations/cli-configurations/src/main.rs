mod utils;

use ::infrastructures::DownloadVideoView;
use ::infrastructures::Id3MetadataWriter;
use ::infrastructures::YtDlpDownloader;
use ::use_cases::boundaries::DownloadVideoInputBoundary;
use ::use_cases::boundaries::DownloadVideoRequestModel;
use ::use_cases::interactors::DownloadVideoInteractor;

use crate::utils::aliases::Fallible;

#[tokio::main]
async fn main() -> Fallible<()> {
    let download_video_view = std::sync::Arc::new(DownloadVideoView::new()?);
    let downloader = std::sync::Arc::new(YtDlpDownloader::new());
    let metadata_writer = std::sync::Arc::new(Id3MetadataWriter::new());

    let download_video_interactor =
        DownloadVideoInteractor::new(download_video_view.clone(), downloader.clone(), metadata_writer.clone());

    let command = ::clap::Command::new("walkman")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            ::clap::Command::new("download-video")
                .arg(
                    ::clap::Arg::new("url")
                        .short('i')
                        .default_value("https://youtu.be/dQw4w9WgXcQ?list=RDdQw4w9WgXcQ")
                        .value_parser(::clap::value_parser!(::std::string::String)),
                )
                .arg(
                    ::clap::Arg::new("directory")
                        .short('o')
                        .default_value(env!("CARGO_WORKSPACE_DIR"))
                        .value_parser(::clap::value_parser!(::std::path::PathBuf)),
                ),
        );

    match command.get_matches().subcommand() {
        Some(("download-video", matches)) => {
            let url = matches
                .get_one::<::std::string::String>("url")
                .expect("Error: Missing required argument `url`");
            let directory = matches
                .get_one::<::std::path::PathBuf>("directory")
                .expect("Error: Missing required argument `directory`");

            let model = DownloadVideoRequestModel {
                url: url.to_owned().into(),
                directory: directory.to_owned().into(),
            };

            download_video_interactor.apply(model).await?;
        },
        _ => unreachable!(),
    }

    Ok(())
}
