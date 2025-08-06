use ::async_trait::async_trait;
use ::domain::ChannelUrl;
use ::domain::PlaylistUrl;
use ::domain::VideoUrl;
use ::futures::prelude::*;
use ::std::ops::Not;
use ::use_cases::gateways::ChannelDownloader;
use ::use_cases::gateways::PlaylistDownloader;
use ::use_cases::gateways::VideoDownloader;
use ::use_cases::models::descriptors::ChannelMetadata;
use ::use_cases::models::descriptors::PartiallyResolvedChannel;
use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
use ::use_cases::models::descriptors::PartiallyResolvedVideo;
use ::use_cases::models::descriptors::PlaylistMetadata;
use ::use_cases::models::descriptors::ResolvedChannel;
use ::use_cases::models::descriptors::ResolvedPlaylist;
use ::use_cases::models::descriptors::ResolvedVideo;
use ::use_cases::models::descriptors::UnresolvedPlaylist;
use ::use_cases::models::descriptors::UnresolvedVideo;
use ::use_cases::models::descriptors::VideoMetadata;
use ::use_cases::models::events::ChannelDownloadCompletedEvent;
use ::use_cases::models::events::ChannelDownloadEvent;
use ::use_cases::models::events::ChannelDownloadProgressUpdatedEvent;
use ::use_cases::models::events::ChannelDownloadStartedEvent;
use ::use_cases::models::events::DiagnosticEvent;
use ::use_cases::models::events::DiagnosticLevel;
use ::use_cases::models::events::PlaylistDownloadCompletedEvent;
use ::use_cases::models::events::PlaylistDownloadEvent;
use ::use_cases::models::events::PlaylistDownloadProgressUpdatedEvent;
use ::use_cases::models::events::PlaylistDownloadStartedEvent;
use ::use_cases::models::events::VideoDownloadCompletedEvent;
use ::use_cases::models::events::VideoDownloadEvent;
use ::use_cases::models::events::VideoDownloadProgressUpdatedEvent;
use ::use_cases::models::events::VideoDownloadStartedEvent;

use crate::utils::aliases::BoxedStream;
use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedPath;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::aliases::MaybeOwnedVec;
use crate::utils::extensions::OptionExt;

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct YtdlpDownloader {
    directory: MaybeOwnedPath,

    #[allow(dead_code)]
    workers: u64,

    per_worker_cooldown: ::std::time::Duration,

    #[builder(skip = ::std::sync::Arc::new(::tokio::sync::Semaphore::new(workers as usize)))]
    worker_pool: ::std::sync::Arc<::tokio::sync::Semaphore>,
}

#[async_trait]
impl VideoDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: VideoUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<DiagnosticEvent>)> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        ::tokio::spawn(async move {
            #[rustfmt::skip]
            let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
                &*url,
                "--quiet",
                "--color", "no_color",
                "--paths", self.directory.to_str().ok()?,
                "--no-playlist",
                "--format", "bestaudio",
                "--extract-audio",
                "--audio-format", "mp3",
                "--output", "%(title)+U.%(ext)s",
                "--newline",
                "--restrict-filenames",
                "--windows-filenames",
                "--abort-on-error",
                "--force-overwrites",
                "--progress",
                "--print", "before_dl:[video-started]%(id)s;%(webpage_url)s;%(title)+U;%(album)s;%(artist)s;%(genre)s",
                "--progress-template", "[video-downloading]%(info.id)s;%(progress.eta)s;%(progress.elapsed)s;%(progress.downloaded_bytes)s;%(progress.total_bytes)s;%(progress.speed)s",
                "--print", "after_move:[video-completed]%(id)s;%(webpage_url)s;%(title)+U;%(album)s;%(artist)s;%(genre)s;%(filepath)+U",
            ])?;

            ::tokio::try_join!(
                async {
                    stdout
                        .filter_map(|line| async { VideoDownloadEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { video_download_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
                async {
                    stderr
                        .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
            )?;

            Ok::<_, ::anyhow::Error>(())
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

#[async_trait]
impl PlaylistDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: PlaylistUrl,
    ) -> Fallible<(BoxedStream<VideoDownloadEvent>, BoxedStream<PlaylistDownloadEvent>, BoxedStream<DiagnosticEvent>)>
    {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        ::tokio::spawn(async move {
            #[rustfmt::skip]
            let (stdout, stderr) = TokioCommandExecutor::execute("yt-dlp", [
                &*url,
                "--quiet",
                "--color", "no_color",
                "--flat-playlist",
                "--yes-playlist",
                "--print", "playlist:[playlist-started:metadata]%(id)s;%(webpage_url)s;%(title)s",
                "--print", "video:[playlist-started:video]%(id)s;%(url)s"
            ])?;

            let (playlist, _) = ::tokio::try_join!(
                async {
                    let event = PlaylistDownloadStartedEvent::from_lines(stdout).await.ok()?;
                    let playlist = event.playlist.clone();

                    playlist_download_events_tx.send(PlaylistDownloadEvent::Started(event))?;

                    Ok(playlist)
                },
                async {
                    stderr
                        .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
            )?;

            let playlist = PartiallyResolvedPlaylistDeduplicator::deduplicate(playlist);

            ::tracing::debug!("Downloaded playlist `{:?}`", playlist);

            let completed_videos = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::new(0));
            let total_videos = playlist.videos.as_deref().map(|videos| videos.len() as u64).unwrap_or_default();

            let videos = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_videos as usize)));

            let videos_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());

            ::tracing::debug!(
                "Downloaded videos `{:?}` (`{}`/`{}`)",
                videos,
                completed_videos.load(::std::sync::atomic::Ordering::Relaxed),
                total_videos
            );

            playlist.videos.as_deref().into_iter().flatten().cloned().for_each(|video| {
                ::tokio::spawn({
                    let this = ::std::sync::Arc::clone(&self);

                    let video_download_events_tx = video_download_events_tx.clone();
                    let playlist_download_events_tx = playlist_download_events_tx.clone();
                    let diagnostic_events_tx = diagnostic_events_tx.clone();

                    let playlist_id = playlist.id.clone();

                    let completed_videos = ::std::sync::Arc::clone(&completed_videos);
                    let videos = ::std::sync::Arc::clone(&videos);
                    let videos_completed_notify = ::std::sync::Arc::clone(&videos_completed_notify);

                    async move {
                        let worker = this.worker_pool.acquire().await?;

                        let (video_download_events, diagnostic_events) =
                            VideoDownloader::download(::std::sync::Arc::clone(&this), video.url.clone().into()).await?;

                        ::tokio::try_join!(
                            async {
                                video_download_events
                                    .map(Ok)
                                    .try_for_each(|event| async {
                                        if let VideoDownloadEvent::Completed(ref event) = event {
                                            completed_videos.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                            videos.lock().await.push(event.video.clone());

                                            let event = PlaylistDownloadProgressUpdatedEvent::builder()
                                                .playlist_id(playlist_id.clone())
                                                .completed_videos(
                                                    completed_videos.load(::std::sync::atomic::Ordering::Relaxed),
                                                )
                                                .total_videos(total_videos)
                                                .build();

                                            playlist_download_events_tx
                                                .send(PlaylistDownloadEvent::ProgressUpdated(event))?;
                                        }

                                        video_download_events_tx.send(event)?;

                                        Ok(())
                                    })
                                    .await
                            },
                            async {
                                diagnostic_events
                                    .map(Ok)
                                    .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                                    .await
                                    .map_err(::anyhow::Error::from)
                            },
                        )?;

                        ::tokio::time::sleep(this.per_worker_cooldown).await;
                        ::core::mem::drop(worker);

                        ::tracing::debug!(
                            "Downloaded videos `{:?}` (`{}`/`{}`)",
                            videos,
                            completed_videos.load(::std::sync::atomic::Ordering::Relaxed),
                            total_videos
                        );

                        if completed_videos.load(::std::sync::atomic::Ordering::Relaxed) == total_videos {
                            videos_completed_notify.notify_one();
                        }

                        Fallible::Ok(())
                    }
                });
            });

            videos_completed_notify.notified().await;

            let videos = ::std::mem::take(&mut *videos.lock().await);
            let videos = videos.is_empty().not().then_some(videos.into());

            let playlist = ResolvedPlaylist::builder()
                .id(playlist.id)
                .url(playlist.url)
                .metadata(playlist.metadata)
                .videos(videos)
                .build();

            ::tracing::debug!("Downloaded playlist `{:?}`", playlist);

            let event = PlaylistDownloadCompletedEvent { playlist };
            playlist_download_events_tx.send(PlaylistDownloadEvent::Completed(event))?;

            Ok::<_, ::anyhow::Error>(())
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

#[async_trait]
impl ChannelDownloader for YtdlpDownloader {
    async fn download(
        self: ::std::sync::Arc<Self>, url: ChannelUrl,
    ) -> Fallible<(
        BoxedStream<VideoDownloadEvent>,
        BoxedStream<PlaylistDownloadEvent>,
        BoxedStream<ChannelDownloadEvent>,
        BoxedStream<DiagnosticEvent>,
    )> {
        let (video_download_events_tx, video_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (playlist_download_events_tx, playlist_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (channel_download_events_tx, channel_download_events_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (diagnostic_events_tx, diagnostic_events_rx) = ::tokio::sync::mpsc::unbounded_channel();

        ::tokio::spawn(async move {
            #[rustfmt::skip]
            let (stdout, stderr) = TokioCommandExecutor::execute_all(&[
                ("yt-dlp", &[
                    &format!("{}/videos", &*url) as &str,
                    "--quiet",
                    "--color", "no_color",
                    "--print", "[channel-started:video]%(id)s;%(webpage_url)s;%(channel_id)s;%(channel_url)s;%(channel)s",
                ]),
                ("yt-dlp", &[
                    &format!("{}/playlists", &*url),
                    "--quiet",
                    "--color", "no_color",
                    "--flat-playlist",
                    "--print", "[channel-started:playlist]%(id)s;%(url)s",
                ]),
            ])?;

            let (channel, _) = ::tokio::try_join!(
                async {
                    let event = ChannelDownloadStartedEvent::from_lines(stdout).await.ok()?;
                    let channel = event.channel.clone();

                    channel_download_events_tx.send(ChannelDownloadEvent::Started(event))?;

                    Ok(channel)
                },
                async {
                    stderr
                        .filter_map(|line| async { DiagnosticEvent::from_line(line) })
                        .map(Ok)
                        .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                        .await
                        .map_err(::anyhow::Error::from)
                },
            )?;

            let channel = PartiallyResolvedChannelDeduplicator::deduplicate(channel);

            ::tracing::debug!("Downloading channel `{:?}`", channel);

            let completed_videos = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::default());
            let total_videos = channel.videos.as_deref().map(|videos| videos.len() as u64).unwrap_or_default();
            let completed_playlists = ::std::sync::Arc::new(::std::sync::atomic::AtomicU64::default());
            let total_playlists = channel
                .playlists
                .as_deref()
                .map(|playlists| playlists.len() as u64)
                .unwrap_or_default();

            let videos = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_videos as usize)));
            let playlists =
                ::std::sync::Arc::new(::tokio::sync::Mutex::new(Vec::with_capacity(total_playlists as usize)));

            let videos_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());
            let playlists_completed_notify = ::std::sync::Arc::new(::tokio::sync::Notify::new());

            ::tracing::debug!(
                "Downloaded videos `{:?}` (`{}`/`{}`)",
                videos,
                completed_videos.load(::std::sync::atomic::Ordering::Relaxed),
                total_videos
            );
            ::tracing::debug!(
                "Downloaded playlists `{:?}` (`{}`/`{}`)",
                playlists,
                completed_playlists.load(::std::sync::atomic::Ordering::Relaxed),
                total_playlists
            );

            ::tokio::try_join!(
                async {
                    channel.videos.as_deref().into_iter().flatten().cloned().for_each(|video| {
                        ::tokio::spawn({
                            let this = ::std::sync::Arc::clone(&self);

                            let video_download_events_tx = video_download_events_tx.clone();
                            let channel_download_events_tx = channel_download_events_tx.clone();
                            let diagnostic_events_tx = diagnostic_events_tx.clone();

                            let channel_id = channel.id.clone();

                            let completed_videos = ::std::sync::Arc::clone(&completed_videos);
                            let completed_playlists = ::std::sync::Arc::clone(&completed_playlists);

                            let videos = ::std::sync::Arc::clone(&videos);
                            let videos_completed_notify = ::std::sync::Arc::clone(&videos_completed_notify);

                            async move {
                                let worker = this.worker_pool.acquire().await?;

                                let (video_download_events, diagnostic_events) =
                                    VideoDownloader::download(::std::sync::Arc::clone(&this), video.url.clone().into())
                                        .await?;

                                ::tokio::try_join!(
                                    async {
                                        video_download_events
                                            .map(Ok)
                                            .try_for_each(|event| async {
                                                if let VideoDownloadEvent::Completed(ref event) = event {
                                                    completed_videos
                                                        .fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                                    videos.lock().await.push(event.video.clone());

                                                    let event = ChannelDownloadProgressUpdatedEvent::builder()
                                                        .channel_id(channel_id.clone())
                                                        .completed_videos(
                                                            completed_videos
                                                                .load(::std::sync::atomic::Ordering::Relaxed),
                                                        )
                                                        .total_videos(total_videos)
                                                        .completed_playlists(
                                                            completed_playlists
                                                                .load(::std::sync::atomic::Ordering::Relaxed),
                                                        )
                                                        .total_playlists(total_playlists)
                                                        .build();

                                                    channel_download_events_tx
                                                        .send(ChannelDownloadEvent::ProgressUpdated(event))?;
                                                }

                                                video_download_events_tx.send(event)?;

                                                Ok(())
                                            })
                                            .await
                                    },
                                    async {
                                        diagnostic_events
                                            .map(Ok)
                                            .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                                            .await
                                            .map_err(::anyhow::Error::from)
                                    },
                                )?;

                                ::tokio::time::sleep(this.per_worker_cooldown).await;
                                ::core::mem::drop(worker);

                                ::tracing::debug!(
                                    "Downloaded video `{:?}` (`{}`/`{}`)",
                                    video,
                                    completed_videos.load(::std::sync::atomic::Ordering::Relaxed),
                                    total_videos
                                );

                                if completed_videos.load(::std::sync::atomic::Ordering::Relaxed) == total_videos {
                                    videos_completed_notify.notify_one();
                                }

                                Ok::<_, ::anyhow::Error>(())
                            }
                        });
                    });

                    Ok::<_, ::anyhow::Error>(())
                },
                async {
                    channel
                        .playlists
                        .as_deref()
                        .into_iter()
                        .flatten()
                        .cloned()
                        .for_each(|playlist| {
                            ::tokio::spawn({
                                let this = ::std::sync::Arc::clone(&self);

                                let video_download_events_tx = video_download_events_tx.clone();
                                let playlist_download_events_tx = playlist_download_events_tx.clone();
                                let channel_download_events_tx = channel_download_events_tx.clone();
                                let diagnostic_events_tx = diagnostic_events_tx.clone();

                                let channel_id = channel.id.clone();

                                let completed_videos = ::std::sync::Arc::clone(&completed_videos);
                                let completed_playlists = ::std::sync::Arc::clone(&completed_playlists);

                                let playlists = ::std::sync::Arc::clone(&playlists);
                                let playlists_completed_notify = ::std::sync::Arc::clone(&playlists_completed_notify);

                                async move {
                                    let worker = this.worker_pool.acquire().await?;

                                    let (video_download_events, playlist_download_events, diagnostic_events) =
                                        PlaylistDownloader::download(
                                            ::std::sync::Arc::clone(&this),
                                            playlist.url.clone().into(),
                                        )
                                        .await?;

                                    ::tokio::try_join!(
                                        async {
                                            video_download_events
                                                .map(Ok)
                                                .try_for_each(|event| async { video_download_events_tx.send(event) })
                                                .await
                                                .map_err(::anyhow::Error::from)
                                        },
                                        async {
                                            playlist_download_events
                                                .map(Ok)
                                                .try_for_each(|event| async {
                                                    if let PlaylistDownloadEvent::Completed(ref event) = event {
                                                        completed_playlists
                                                            .fetch_add(1, ::std::sync::atomic::Ordering::Relaxed);
                                                        playlists.lock().await.push(event.playlist.clone());

                                                        let event = ChannelDownloadProgressUpdatedEvent::builder()
                                                            .channel_id(channel_id.clone())
                                                            .completed_videos(
                                                                completed_videos
                                                                    .load(::std::sync::atomic::Ordering::Relaxed),
                                                            )
                                                            .total_videos(total_videos)
                                                            .completed_playlists(
                                                                completed_playlists
                                                                    .load(::std::sync::atomic::Ordering::Relaxed),
                                                            )
                                                            .total_playlists(total_playlists)
                                                            .build();

                                                        channel_download_events_tx
                                                            .send(ChannelDownloadEvent::ProgressUpdated(event))?;
                                                    }

                                                    playlist_download_events_tx.send(event)?;

                                                    Ok(())
                                                })
                                                .await
                                        },
                                        async {
                                            diagnostic_events
                                                .map(Ok)
                                                .try_for_each(|event| async { diagnostic_events_tx.send(event) })
                                                .await
                                                .map_err(::anyhow::Error::from)
                                        },
                                    )?;

                                    ::tokio::time::sleep(this.per_worker_cooldown).await;
                                    ::core::mem::drop(worker);

                                    ::tracing::debug!(
                                        "Downloaded playlist `{:?}` (`{}`/`{}`)",
                                        playlist,
                                        completed_playlists.load(::std::sync::atomic::Ordering::Relaxed),
                                        total_playlists
                                    );

                                    if completed_playlists.load(::std::sync::atomic::Ordering::Relaxed)
                                        == total_playlists
                                    {
                                        playlists_completed_notify.notify_one();
                                    }

                                    Ok::<_, ::anyhow::Error>(())
                                }
                            });
                        });

                    Ok::<_, ::anyhow::Error>(())
                },
            )?;

            ::tokio::join!(videos_completed_notify.notified(), playlists_completed_notify.notified(),);

            let videos = ::std::mem::take(&mut *videos.lock().await);
            let videos = videos.is_empty().not().then_some(videos.into());

            let playlists = ::std::mem::take(&mut *playlists.lock().await);
            let playlists = playlists.is_empty().not().then_some(playlists.into());

            let channel = ResolvedChannel::builder()
                .id(channel.id)
                .url(channel.url)
                .metadata(channel.metadata)
                .videos(videos)
                .playlists(playlists)
                .build();

            ::tracing::debug!("Downloaded channel `{:?}`", channel);

            let event = ChannelDownloadCompletedEvent { channel };
            channel_download_events_tx.send(ChannelDownloadEvent::Completed(event))?;

            Ok::<_, ::anyhow::Error>(())
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(video_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(playlist_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(channel_download_events_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(diagnostic_events_rx)),
        ))
    }
}

trait CommandExecutor {
    fn execute<Program, Args>(
        program: Program, args: Args,
    ) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>;

    fn execute_all<Program, Arg>(
        commands: &[(Program, &[Arg])],
    ) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Arg: AsRef<::std::ffi::OsStr>,
    {
        let mut stdouts = Vec::with_capacity(commands.len());
        let mut stderrs = Vec::with_capacity(commands.len());

        commands
            .iter()
            .map(|(program, args)| (program, args.iter()))
            .filter_map(|(program, args)| Self::execute(program, args).ok())
            .for_each(|(stdout, stderr)| {
                stdouts.push(stdout);
                stderrs.push(stderr);
            });

        Ok((
            ::std::boxed::Box::pin(::futures::stream::select_all(stdouts)),
            ::std::boxed::Box::pin(::futures::stream::select_all(stderrs)),
        ))
    }
}

struct TokioCommandExecutor;

impl CommandExecutor for TokioCommandExecutor {
    fn execute<Program, Args>(
        program: Program, args: Args,
    ) -> Fallible<(BoxedStream<MaybeOwnedString>, BoxedStream<MaybeOwnedString>)>
    where
        Program: AsRef<::std::ffi::OsStr>,
        Args: IntoIterator,
        Args::Item: AsRef<::std::ffi::OsStr>,
    {
        use ::tokio::io::AsyncBufReadExt as _;

        let (stdout_tx, stdout_rx) = ::tokio::sync::mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = ::tokio::sync::mpsc::unbounded_channel();

        let mut process = ::tokio::process::Command::new(program)
            .args(args)
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().ok()?;
        let stderr = process.stderr.take().ok()?;

        ::tokio::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stdout).lines();

            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| line.to_owned().into())
                .map(Ok)
                .try_for_each(|line| async { stdout_tx.send(line) })
                .await
        });

        ::tokio::spawn(async move {
            let lines = ::tokio::io::BufReader::new(stderr).lines();

            ::tokio_stream::wrappers::LinesStream::new(lines)
                .filter_map(|line| async move { line.ok() })
                .map(|line| line.to_owned().into())
                .map(Ok)
                .try_for_each(|line| async { stderr_tx.send(line) })
                .await
        });

        Ok((
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stdout_rx)),
            ::std::boxed::Box::pin(::tokio_stream::wrappers::UnboundedReceiverStream::new(stderr_rx)),
        ))
    }
}

trait Deduplicator<Artifact> {
    fn deduplicate(artifact: Artifact) -> Artifact;
}

struct PartiallyResolvedPlaylistDeduplicator;

impl Deduplicator<PartiallyResolvedPlaylist> for PartiallyResolvedPlaylistDeduplicator {
    fn deduplicate(playlist: PartiallyResolvedPlaylist) -> PartiallyResolvedPlaylist {
        PartiallyResolvedPlaylist {
            videos: playlist.videos.map(|videos| {
                videos
                    .iter()
                    .cloned()
                    .map(|video| (video.id.clone(), video))
                    .collect::<::indexmap::IndexMap<_, _>>()
                    .into_values()
                    .collect::<Vec<_>>()
                    .into()
            }),
            ..playlist
        }
    }
}

struct PartiallyResolvedChannelDeduplicator;

impl Deduplicator<PartiallyResolvedChannel> for PartiallyResolvedChannelDeduplicator {
    fn deduplicate(channel: PartiallyResolvedChannel) -> PartiallyResolvedChannel {
        PartiallyResolvedChannel {
            videos: channel.videos.map(|videos| {
                videos
                    .iter()
                    .cloned()
                    .map(|video| (video.id.clone(), video))
                    .collect::<::indexmap::IndexMap<_, _>>()
                    .into_values()
                    .collect::<Vec<_>>()
                    .into()
            }),
            playlists: channel.playlists.map(|playlists| {
                playlists
                    .iter()
                    .cloned()
                    .map(|playlist| (playlist.id.clone(), playlist))
                    .collect::<::indexmap::IndexMap<_, _>>()
                    .into_values()
                    .collect::<Vec<_>>()
                    .into()
            }),
            ..channel
        }
    }
}

trait FromYtdlpLine: ::core::marker::Send + ::core::marker::Sync {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized;
}

impl FromYtdlpLine for VideoDownloadEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        ::tracing::debug!("Parsing line `{}` as `VideoDownloadEvent`", line.as_ref());

        let line = line.as_ref();

        VideoDownloadProgressUpdatedEvent::from_line(line)
            .map(Self::ProgressUpdated)
            .or(VideoDownloadStartedEvent::from_line(line).map(Self::Started))
            .or(VideoDownloadCompletedEvent::from_line(line).map(Self::Completed))
    }
}

impl FromYtdlpLine for VideoDownloadStartedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-started]")?.split(';');
        let [id, url, title, album, artists, genres] = YtdlpAttributes::parse(attrs)?.into();

        ::tracing::debug!("Parsed line `{}` as `VideoDownloadStartedEvent`", line.as_ref());

        Some(
            Self::builder()
                .video(
                    PartiallyResolvedVideo::builder()
                        .id(id.singlevalued()?)
                        .url(url.singlevalued()?)
                        .metadata(
                            VideoMetadata::builder()
                                .title(title.singlevalued())
                                .album(album.singlevalued())
                                .artists(artists.multivalued())
                                .genres(genres.multivalued())
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
    }
}

impl FromYtdlpLine for VideoDownloadProgressUpdatedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-downloading]")?.split(';');
        let [id, eta, elapsed, downloaded_bytes, total_bytes, bytes_per_second] = YtdlpAttributes::parse(attrs)?.into();

        ::tracing::debug!("Parsed line `{}` as `VideoDownloadProgressUpdatedEvent`", line.as_ref());

        Some(
            Self::builder()
                .video_id(id.singlevalued()?)
                .eta(::std::time::Duration::from_secs(eta.singlevalued()?.parse().ok()?))
                .elapsed(::std::time::Duration::try_from_secs_f64(elapsed.singlevalued()?.parse().ok()?).ok()?)
                .downloaded_bytes(downloaded_bytes.singlevalued()?.parse().ok()?)
                .total_bytes(total_bytes.singlevalued()?.parse().ok()?)
                .bytes_per_second(bytes_per_second.singlevalued()?.parse::<f64>().ok()?.floor() as u64)
                .build(),
        )
    }
}

impl FromYtdlpLine for VideoDownloadCompletedEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        let attrs = line.as_ref().strip_prefix("[video-completed]")?.split(';');
        let [id, url, title, album, artists, genres, path] = YtdlpAttributes::parse(attrs)?.into();

        ::tracing::debug!("Parsed line `{}` as `VideoDownloadCompletedEvent`", line.as_ref());

        Some(
            Self::builder()
                .video(
                    ResolvedVideo::builder()
                        .id(id.singlevalued()?)
                        .url(url.singlevalued()?)
                        .metadata(
                            VideoMetadata::builder()
                                .title(title.singlevalued())
                                .album(album.singlevalued())
                                .artists(artists.multivalued())
                                .genres(genres.multivalued())
                                .build(),
                        )
                        .path(match path.singlevalued()? {
                            MaybeOwnedString::Borrowed(path) => MaybeOwnedPath::Borrowed(path.as_ref()),
                            MaybeOwnedString::Owned(path) => MaybeOwnedPath::Owned(path.into()),
                        })
                        .build(),
                )
                .build(),
        )
    }
}

impl FromYtdlpLine for DiagnosticEvent {
    fn from_line<Line>(line: Line) -> Option<Self>
    where
        Line: AsRef<str>,
        Self: Sized,
    {
        ::tracing::debug!("Parsing line `{}` as `DiagnosticEvent`", line.as_ref());

        let attrs = line.as_ref().splitn(2, ':');
        let [level, message] = YtdlpAttributes::parse(attrs)?.into();

        ::tracing::debug!("Parsed line `{}` as `DiagnosticEvent`", line.as_ref());

        Some(
            Self::builder()
                .level(match level.singlevalued()?.as_ref() {
                    "WARNING" => DiagnosticLevel::Warning,
                    "ERROR" => DiagnosticLevel::Error,
                    _ => return None,
                })
                .message(message.singlevalued()?)
                .build(),
        )
    }
}

#[async_trait]
trait FromYtdlpLines: ::core::marker::Send + ::core::marker::Sync {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized;
}

#[async_trait]
impl FromYtdlpLines for PlaylistDownloadStartedEvent {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized,
    {
        let (mut playlist_id, mut playlist_url, mut playlist_title) = (None, None, None);
        let mut videos = Vec::new();

        ::futures::pin_mut!(lines);

        while let Some(line) = lines.next().await {
            ::tracing::debug!("Parsing line `{}` as `PlaylistDownloadStartedEvent`", line.as_ref());

            if let Some(line) = line.as_ref().strip_prefix("[playlist-started:video]") {
                let attrs = line.split(';');
                let [id, url] = YtdlpAttributes::parse(attrs)?.into();

                let video = UnresolvedVideo::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .build();

                videos.push(video);
            } else if let Some(line) = line.as_ref().strip_prefix("[playlist-started:metadata]") {
                let attrs = line.split(';');
                let [id, url, title] = YtdlpAttributes::parse(attrs)?.into();

                playlist_id = id.singlevalued();
                playlist_url = url.singlevalued();
                playlist_title = title.singlevalued();
            }
        }

        let playlist = PartiallyResolvedPlaylist::builder()
            .id(playlist_id?)
            .url(playlist_url?)
            .metadata(PlaylistMetadata::builder().title(playlist_title).build())
            .videos(videos.is_empty().not().then(|| videos.into()))
            .build();

        Some(Self { playlist })
    }
}

#[async_trait]
impl FromYtdlpLines for ChannelDownloadStartedEvent {
    async fn from_lines<Lines, Line>(lines: Lines) -> Option<Self>
    where
        Lines: ::futures::Stream<Item = Line> + ::core::marker::Send,
        Line: AsRef<str>,
        Self: Sized,
    {
        let (mut channel_id, mut channel_url, mut channel_title) = (None, None, None);
        let mut videos = Vec::new();
        let mut playlists = Vec::new();

        ::futures::pin_mut!(lines);

        while let Some(line) = lines.next().await {
            ::tracing::debug!("Parsing line `{}` as `ChannelDownloadStartedEvent`", line.as_ref());

            if let Some(line) = line.as_ref().strip_prefix("[channel-started:video]") {
                let attrs = line.split(';');
                let [video_id, video_url, channel_id_, channel_url_, channel_title_] =
                    YtdlpAttributes::parse(attrs)?.into();

                let video = UnresolvedVideo::builder()
                    .id(video_id.singlevalued()?)
                    .url(video_url.singlevalued()?)
                    .build();

                videos.push(video);

                channel_id = channel_id_.singlevalued();
                channel_url = channel_url_.singlevalued();
                channel_title = channel_title_.singlevalued();
            } else if let Some(line) = line.as_ref().strip_prefix("[channel-started:playlist]") {
                let attrs = line.split(';');
                let [id, url] = YtdlpAttributes::parse(attrs)?.into();

                let playlist = UnresolvedPlaylist::builder()
                    .id(id.singlevalued()?)
                    .url(url.singlevalued()?)
                    .build();

                playlists.push(playlist);
            }
        }

        let channel = PartiallyResolvedChannel::builder()
            .id(channel_id?)
            .url(channel_url?)
            .metadata(ChannelMetadata::builder().title(channel_title).build())
            .videos(videos.is_empty().not().then(|| videos.into()))
            .playlists(playlists.is_empty().not().then(|| playlists.into()))
            .build();

        Some(Self { channel })
    }
}

#[derive(Debug, Clone)]
struct YtdlpAttribute<'a>(&'a str);

impl<'a> YtdlpAttribute<'a> {
    fn singlevalued(self) -> Option<MaybeOwnedString> {
        match self.0.trim() {
            "NA" => None,
            attr => Some(attr.to_owned().into()),
        }
    }

    fn multivalued(self) -> Option<MaybeOwnedVec<MaybeOwnedString>> {
        let attrs = self
            .0
            .split(',')
            .map(YtdlpAttribute)
            .filter_map(Self::singlevalued)
            .collect::<Vec<_>>();

        Some(attrs.into())
    }
}

#[derive(Debug, Clone)]
struct YtdlpAttributes<'a, const N: usize>([YtdlpAttribute<'a>; N]);

impl<'a, const N: usize> From<YtdlpAttributes<'a, N>> for [YtdlpAttribute<'a>; N] {
    fn from(outer: YtdlpAttributes<'a, N>) -> Self {
        outer.0
    }
}

impl<'a, const N: usize> YtdlpAttributes<'a, N> {
    fn parse<Attrs>(attrs: Attrs) -> Option<Self>
    where
        Attrs: Iterator<Item = &'a str>,
    {
        let attrs = attrs.map(YtdlpAttribute).collect::<Vec<_>>().try_into().ok()?;

        Some(Self(attrs))
    }
}
