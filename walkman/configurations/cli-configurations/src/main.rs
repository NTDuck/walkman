pub(crate) mod utils;

use ::infrastructures::boundaries::AggregateView;
use ::infrastructures::gateways::downloaders::YtdlpDownloader;
use ::infrastructures::gateways::postprocessors::AlbumNamingPolicy;
use ::infrastructures::gateways::postprocessors::ArtistsNamingPolicy;
use ::infrastructures::gateways::postprocessors::Id3MetadataWriter;
use ::infrastructures::gateways::repositories::BincodeSerializer;
use ::infrastructures::gateways::repositories::CompressedSerializedFilesystemResourcesRepository;
use ::infrastructures::gateways::repositories::Compressor;
use ::infrastructures::gateways::repositories::Flate2Compressor;
use ::infrastructures::gateways::repositories::Serializer;
use ::use_cases::boundaries::Accept;
use ::use_cases::boundaries::DownloadChannelOutputBoundary;
use ::use_cases::boundaries::DownloadChannelRequestModel;
use ::use_cases::boundaries::DownloadPlaylistOutputBoundary;
use ::use_cases::boundaries::DownloadPlaylistRequestModel;
use ::use_cases::boundaries::DownloadVideoOutputBoundary;
use ::use_cases::boundaries::DownloadVideoRequestModel;
use ::use_cases::boundaries::UpdateMediaOutputBoundary;
use ::use_cases::boundaries::UpdateMediaRequestModel;
use ::use_cases::gateways::ChannelDownloader;
use ::use_cases::gateways::PlaylistDownloader;
use ::use_cases::gateways::PostProcessor;
use ::use_cases::gateways::UrlRepository;
use ::use_cases::gateways::VideoDownloader;
use ::use_cases::interactors::DownloadChannelInteractor;
use ::use_cases::interactors::DownloadPlaylistInteractor;
use ::use_cases::interactors::DownloadVideoInteractor;
use ::use_cases::interactors::UpdateMediaInteractor;
use ::use_cases::models::descriptors::ResolvedChannel;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;

use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::extensions::OptionExt;

#[tokio::main]
async fn main() -> Fallible<()> {
    let logger = ::tracing_appender::rolling::minutely("logs", "cli.log");
    let (logger, _logger_guard) = ::tracing_appender::non_blocking(logger);

    // Logger
    ::tracing_subscriber::fmt()
        .with_writer(logger)
        .with_env_filter(::tracing_subscriber::EnvFilter::try_from_default_env()?)
        .with_ansi(false)
        .init();

    #[rustfmt::skip]
    let command = ::clap::command!("walkman")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(::clap::command!("download-video")
            .alias("download")
            .arg(::clap::arg!(-i --url <URL>)
                .value_parser(::clap::value_parser!(::std::string::String))))
        .subcommand(::clap::command!("download-playlist")
            .arg(::clap::arg!(-i --url <URL>)
                .value_parser(::clap::value_parser!(::std::string::String))))
        .subcommand(::clap::command!("download-channel")
            .arg(::clap::arg!(-i --url <URL>)
                .value_parser(::clap::value_parser!(::std::string::String))))
        .subcommand(::clap::command!("update-media")
            .alias("update"))
        .arg(::clap::arg!(-o --directory <FOLDER>)
            .value_parser(::clap::value_parser!(::std::path::PathBuf)))
        .arg(::clap::arg!(--"video-urls-path" [FILE])
            .value_parser(::clap::value_parser!(::std::path::PathBuf)))
        .arg(::clap::arg!(--"playlist-urls-path" [FILE])
            .value_parser(::clap::value_parser!(::std::path::PathBuf)))
        .arg(::clap::arg!(--"channel-urls-path" [FILE])
            .value_parser(::clap::value_parser!(::std::path::PathBuf)))
        .arg(::clap::arg!(-N --workers [NUMBER])
            .value_parser(::clap::value_parser!(u64)))
        .arg(::clap::arg!(--"per-worker-cooldown" [MILLISECONDS])
            .default_value("0")
            .value_parser(::clap::value_parser!(u64)))
        .arg(::clap::arg!(--"set-video-album-as" [POLICY])
            .default_value("playlist-title")
            .value_parser(["video-album", "playlist-title"]))
        .arg(::clap::arg!(--"set-video-artists-as" [POLICY])
            .default_value("video-artists-and-channel-title")
            .value_parser(["video-artists", "channel-title", "video-artists-and-channel-title"]));

    let matches = command.get_matches();

    // Arguments
    let directory: MaybeOwnedPath = matches.get_one::<::std::path::PathBuf>("directory")
        .ok()?.to_owned().into();

    let video_urls_path: MaybeOwnedPath = matches.get_one::<::std::path::PathBuf>("video-urls-path").cloned()
        .unwrap_or_else(|| directory.join("persist/video-urls.bin"))
        .to_owned().into();
    let playlist_urls_path: MaybeOwnedPath = matches.get_one::<::std::path::PathBuf>("playlist-urls-path").cloned()
        .unwrap_or_else(|| directory.join("persist/playlist-urls.bin"))
        .to_owned().into();
    let channel_urls_path: MaybeOwnedPath = matches.get_one::<::std::path::PathBuf>("channel-urls-path").cloned()
        .unwrap_or_else(|| directory.join("persist/channel-urls.bin"))
        .to_owned().into();

    let workers = matches.get_one::<u64>("workers").ok().copied()
        .unwrap_or_else(|_| ::num_cpus::get() as u64);
    let per_worker_cooldown = matches.get_one::<u64>("per-worker-cooldown")
        .map(|cooldown| ::std::time::Duration::from_millis(*cooldown))
        .ok()?;

    let album_naming_policy = match matches.get_one::<::std::string::String>("set-video-album-as").ok()? as &str {
        "video-album" => AlbumNamingPolicy::UseVideoAlbum,
        "playlist-title" => AlbumNamingPolicy::UsePlaylistTitle,
        _ => panic!(),
    };
    let artists_naming_policy = match matches.get_one::<::std::string::String>("set-video-artists-as").ok()? as &str {
        "video-artists" => ArtistsNamingPolicy::UseOnlyVideoArtists,
        "channel-title" => ArtistsNamingPolicy::UseOnlyChannelTitle,
        "video-artists-and-channel-title" => ArtistsNamingPolicy::UseBothVideoArtistsAndChannelTitle,
        _ => panic!(),
    };

    // Boundaries
    let view = ::std::sync::Arc::new(AggregateView::builder().build());

    // Gateways
    let serializer = ::std::sync::Arc::new(BincodeSerializer::builder()
        .configurations(::bincode::config::standard())
        .build());
    let compressor = ::std::sync::Arc::new(Flate2Compressor::builder()
        .level(::flate2::Compression::default())
        .build());
    let urls = ::std::sync::Arc::new(
        CompressedSerializedFilesystemResourcesRepository::builder()
            .serializer(::std::sync::Arc::clone(&serializer) as ::std::sync::Arc<dyn Serializer<::std::collections::HashSet<MaybeOwnedString, ::ahash::RandomState>>>)
            .compressor(::std::sync::Arc::clone(&compressor) as ::std::sync::Arc<dyn Compressor>)
            .video_urls_path(video_urls_path)
            .playlist_urls_path(playlist_urls_path)
            .channel_urls_path(channel_urls_path)
            .build()
            .await?,
    );

    let downloader = ::std::sync::Arc::new(
        YtdlpDownloader::builder()
            .directory(directory)
            .workers(workers)
            .per_worker_cooldown(per_worker_cooldown)
            .build(),
    );

    let metadata_writer = ::std::sync::Arc::new(
        Id3MetadataWriter::builder()
            .album_naming_policy(album_naming_policy)
            .artists_naming_policy(artists_naming_policy)
            .build(),
    );

    let video_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>> =
        vec![::std::sync::Arc::clone(&metadata_writer) as ::std::sync::Arc<dyn PostProcessor<ResolvedVideo>>];
    let playlist_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>> =
        vec![::std::sync::Arc::clone(&metadata_writer) as ::std::sync::Arc<dyn PostProcessor<ResolvedPlaylist>>];
    let channel_postprocessors: Vec<::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>> =
        vec![::std::sync::Arc::clone(&metadata_writer) as ::std::sync::Arc<dyn PostProcessor<ResolvedChannel>>];

    // Interactors
    let download_video_interactor: std::sync::Arc<DownloadVideoInteractor> = ::std::sync::Arc::new(
        DownloadVideoInteractor::builder()
            .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn DownloadVideoOutputBoundary>)
            .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
            .downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn VideoDownloader>)
            .postprocessors(video_postprocessors.clone())
            .build(),
    );
    let download_playlist_interactor = ::std::sync::Arc::new(
        DownloadPlaylistInteractor::builder()
            .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn DownloadPlaylistOutputBoundary>)
            .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
            .downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn PlaylistDownloader>)
            .postprocessors(playlist_postprocessors.clone())
            .build(),
    );
    let download_channel_interactor = ::std::sync::Arc::new(
        DownloadChannelInteractor::builder()
            .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn DownloadChannelOutputBoundary>)
            .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
            .downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn ChannelDownloader>)
            .postprocessors(channel_postprocessors.clone())
            .build(),
    );
    let update_media_interactor = ::std::sync::Arc::new(
        UpdateMediaInteractor::builder()
            .view(::std::sync::Arc::clone(&view) as ::std::sync::Arc<dyn UpdateMediaOutputBoundary>)
            .urls(::std::sync::Arc::clone(&urls) as ::std::sync::Arc<dyn UrlRepository>)
            .video_downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn VideoDownloader>)
            .playlist_downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn PlaylistDownloader>)
            .channel_downloader(::std::sync::Arc::clone(&downloader) as ::std::sync::Arc<dyn ChannelDownloader>)
            .video_postprocessors(video_postprocessors.clone())
            .playlist_postprocessors(playlist_postprocessors.clone())
            .channel_postprocessors(channel_postprocessors.clone())
            .build(),
    );

    // Routing
    match matches.subcommand() {
        Some(("download-video", matches)) => {
            let url = matches.get_one::<::std::string::String>("url").ok()?.to_owned();
            let request = DownloadVideoRequestModel::builder()
                .url(url)
                .build();
            download_video_interactor.accept(request).await?;
        },
        Some(("download-playlist", matches)) => {
            let url = matches.get_one::<::std::string::String>("url").ok()?.to_owned();
            let request = DownloadPlaylistRequestModel::builder()
                .url(url)
                .build();
            download_playlist_interactor.accept(request).await?;
        },
        Some(("download-channel", matches)) => {
            let url = matches.get_one::<::std::string::String>("url").ok()?.to_owned();
            let request = DownloadChannelRequestModel::builder()
                .url(url)
                .build();
            download_channel_interactor.accept(request).await?;
        },
        Some(("update", _)) => {
            let request = UpdateMediaRequestModel;
            update_media_interactor.accept(request).await?;
        },

        _ => unreachable!(),
    }

    Ok(())
}
