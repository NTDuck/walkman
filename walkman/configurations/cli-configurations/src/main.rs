pub(crate) mod utils;

use infrastructures::boundaries::AggregateView;
use ::infrastructures::gateways::downloaders::YtdlpDownloader;
use ::infrastructures::gateways::postprocessors::AlbumNamingPolicy;
use ::infrastructures::gateways::postprocessors::Id3MetadataWriter;
use ::infrastructures::gateways::repositories::FilesystemResourcesRepository;
use ::use_cases::boundaries::Accept;
use use_cases::boundaries::DownloadPlaylistOutputBoundary;
use ::use_cases::boundaries::DownloadPlaylistRequestModel;
use use_cases::boundaries::DownloadVideoOutputBoundary;
use ::use_cases::boundaries::DownloadVideoRequestModel;
use use_cases::gateways::PlaylistDownloader;
use ::use_cases::gateways::PostProcessor;
use use_cases::gateways::UrlRepository;
use use_cases::gateways::VideoDownloader;
use ::use_cases::interactors::DownloadPlaylistInteractor;
use ::use_cases::interactors::DownloadVideoInteractor;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;

use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::extensions::OptionExt;

#[tokio::main]
async fn main() -> Fallible<()> {
    let command = ::clap::Command::new("walkman")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            ::clap::Command::new("download-video").arg(
                ::clap::Arg::new("url")
                    .short('i')
                    .required(true)
                    .value_parser(::clap::value_parser!(::std::string::String)),
            ),
        )
        .subcommand(
            ::clap::Command::new("download-playlist").arg(
                ::clap::Arg::new("url")
                    .short('i')
                    .required(true)
                    .value_parser(::clap::value_parser!(::std::string::String)),
            ),
        )
        .subcommand(
            ::clap::Command::new("download-channel").arg(
                ::clap::Arg::new("url")
                    .short('i')
                    .required(true)
                    .value_parser(::clap::value_parser!(::std::string::String)),
            ),
        )
        .subcommand(
            ::clap::Command::new("update")
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
                .value_parser(::clap::value_parser!(u64)),
        )
        .arg(
            ::clap::Arg::new("per-worker-cooldown")
                .default_value("0")
                .value_parser(::clap::value_parser!(u64)),
        )
        .arg(
            ::clap::Arg::new("set-album-as")
                .default_value("playlist-title")
                .value_parser(["video-album", "playlist-title"]),
        );

    let matches = command.get_matches();
    
    let directory: MaybeOwnedPath = matches.get_one::<::std::path::PathBuf>("directory").ok()?.to_owned().into();

    let view = ::std::sync::Arc::new(AggregateView::builder().build());

    let urls = ::std::sync::Arc::new(FilesystemResourcesRepository::builder()
        .video_urls_path(directory.join("video-urls.txt"))
        .playlist_urls_path(directory.join("playlist-urls.txt"))
        .channel_urls_path(directory.join("channel-urls.txt"))
        .build());

    let downloader = ::std::sync::Arc::new(YtdlpDownloader::builder()
        .directory(directory)
        .workers(matches
            .get_one::<u64>("workers")
            .ok()
            .copied()
            .unwrap_or_else(|_| ::num_cpus::get() as u64))
        .per_worker_cooldown(matches
            .get_one::<u64>("per-worker-cooldown")
            .map(|cooldown| ::std::time::Duration::from_millis(*cooldown))
            .ok()?)
        .build());

    let metadata_writer = ::std::sync::Arc::new(Id3MetadataWriter::builder()
        .album_naming_policy(match matches.get_one::<::std::string::String>("set-album-as").ok()?.as_ref() {
            "video-album" => AlbumNamingPolicy::UseVideoAlbum,
            "playlist-title" => AlbumNamingPolicy::UsePlaylistTitle,
            _ => panic!(),
        })
        .build());

    let video_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>> =
        vec![::std::sync::Arc::clone(&metadata_writer) as ::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>];
    let playlist_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>> =
        vec![::std::sync::Arc::clone(&metadata_writer) as ::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>];

    let download_video_interactor: std::sync::Arc<DownloadVideoInteractor> = ::std::sync::Arc::new(DownloadVideoInteractor::builder()
        .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn DownloadVideoOutputBoundary>)
        .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
        .downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn VideoDownloader>)
        .postprocessors(video_postprocessors)
        .build());
    let download_playlist_interactor = ::std::sync::Arc::new(DownloadPlaylistInteractor::builder()
        .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>)
        .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
        .downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn PlaylistDownloader>)
        .postprocessors(playlist_postprocessors)
        .build());

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
