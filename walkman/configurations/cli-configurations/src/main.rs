pub(crate) mod utils;

use ::infrastructures::boundaries::DownloadPlaylistView;
use ::infrastructures::boundaries::DownloadVideoView;
use ::infrastructures::gateways::downloaders::YtdlpConfigurations;
use ::infrastructures::gateways::downloaders::YtdlpDownloader;
use ::infrastructures::gateways::postprocessors::AlbumNamingPolicy;
use ::infrastructures::gateways::postprocessors::Id3MetadataWriter;
use ::infrastructures::gateways::postprocessors::Id3MetadataWriterConfigurations;
use ::use_cases::boundaries::Accept;
use ::use_cases::boundaries::DownloadPlaylistRequestModel;
use ::use_cases::boundaries::DownloadVideoRequestModel;
use ::use_cases::gateways::PostProcessor;
use ::use_cases::interactors::DownloadPlaylistInteractor;
use ::use_cases::interactors::DownloadVideoInteractor;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;

use crate::utils::aliases::Fallible;
use crate::utils::extensions::OptionExt;

#[tokio::main]
async fn main() -> Fallible<()> {
    let command = ::clap::Command::new("walkman")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            ::clap::Command::new("download-video")
                .arg(
                    ::clap::Arg::new("url")
                        .short('i')
                        .required(true)
                        .value_parser(::clap::value_parser!(::std::string::String)),
                )
        )
        .subcommand(
            ::clap::Command::new("download-playlist")
                .arg(
                    ::clap::Arg::new("url")
                        .short('i')
                        .required(true)
                        .value_parser(::clap::value_parser!(::std::string::String)),
                )
        )
        .arg(
            ::clap::Arg::new("directory")
                .short('o')
                .default_value(env!("CARGO_WORKSPACE_DIR"))
                .value_parser(::clap::value_parser!(::std::path::PathBuf)),
        )
        .arg(
            ::clap::Arg::new("workers")
                .short('N')
                .required(false)
                .value_parser(::clap::value_parser!(u64))
        )
        .arg(
            ::clap::Arg::new("per-worker-cooldown")
                .default_value("0")
                .value_parser(::clap::value_parser!(u64))
        )
        .arg(
            ::clap::Arg::new("set-album-as")
                .default_value("playlist-title")
                .value_parser(["video-album", "playlist-title"])
        );

    let matches = command.get_matches();

    let download_video_view = ::std::sync::Arc::new(DownloadVideoView::new()?);
    let download_playlist_view = ::std::sync::Arc::new(DownloadPlaylistView::new()?);

    let downloader = ::std::sync::Arc::new(YtdlpDownloader::new(
        YtdlpConfigurations {
            directory: matches.get_one::<::std::path::PathBuf>("directory").ok()?.to_owned().into(),
            workers: matches.get_one::<u64>("workers").ok()
                .map(|workers| *workers)
                .unwrap_or_else(|_| ::num_cpus::get() as u64),
            per_worker_cooldown: matches.get_one::<u64>("per-worker-cooldown")
                .map(|cooldown| ::std::time::Duration::from_millis(*cooldown)).ok()?,
        },
    ));
    let metadata_writer = ::std::sync::Arc::new(Id3MetadataWriter::new(
        Id3MetadataWriterConfigurations {
            policy: match matches.get_one::<::std::string::String>("set-album-as").ok()?.as_ref() {
                "video-album" => AlbumNamingPolicy::UseVideoAlbum,
                "playlist-title" => AlbumNamingPolicy::UsePlaylistTitle,
                _ => panic!(),
            },
        },
    ));

    let video_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>> = vec![
        metadata_writer.clone(),
    ];

    let playlist_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>> = vec![
        metadata_writer.clone(),
    ];

    let download_video_interactor: std::sync::Arc<DownloadVideoInteractor> = ::std::sync::Arc::new(DownloadVideoInteractor::new(download_video_view.clone(), downloader.clone(), video_postprocessors.into()));
    let download_playlist_interactor = ::std::sync::Arc::new(DownloadPlaylistInteractor::new(download_playlist_view.clone(), downloader.clone(), playlist_postprocessors.into()));

    match matches.subcommand() {
        Some(("download-video", matches)) => {
            let url = matches.get_one::<::std::string::String>("url").ok()?.to_owned().into();
            
            let request = DownloadVideoRequestModel { url };
            download_video_interactor.accept(request).await?;

        },

        Some(("download-playlist", matches)) => {
            let url = matches.get_one::<::std::string::String>("url").ok()?.to_owned().into();
            
            let request = DownloadPlaylistRequestModel { url };
            download_playlist_interactor.accept(request).await?;
        },

        _ => unreachable!(),
    }

    Ok(())
}
