// use ::async_trait::async_trait;
// use ::derive_new::new;
// use ::domain::PlaylistMetadata;
// use ::domain::VideoMetadata;
// use ::use_cases::boundaries::Activate;
// use ::use_cases::boundaries::Update;
// use ::use_cases::gateways::PostProcessor;
// use ::use_cases::models::descriptors::PartiallyResolvedPlaylist;
// use ::use_cases::models::descriptors::PartiallyResolvedVideo;
// use ::use_cases::models::descriptors::ResolvedPlaylist;
// use ::use_cases::models::descriptors::ResolvedVideo;
// use ::use_cases::models::descriptors::UnresolvedVideo;
// use ::use_cases::models::events::DiagnosticEvent;
// use ::use_cases::models::events::DiagnosticLevel;
// use ::use_cases::models::events::PlaylistDownloadCompletedEvent;
// use ::use_cases::models::events::PlaylistDownloadEvent;
// use ::use_cases::models::events::PlaylistDownloadProgressUpdatedEvent;
// use ::use_cases::models::events::PlaylistDownloadStartedEvent;
// use ::use_cases::models::events::VideoDownloadCompletedEvent;
// use ::use_cases::models::events::VideoDownloadEvent;
// use ::use_cases::models::events::VideoDownloadProgressUpdatedEvent;
// use ::use_cases::models::events::VideoDownloadStartedEvent;

// use crate::lazy_progress_style;
// use crate::utils::aliases::BoxedStream;
// use crate::utils::aliases::Fallible;
// use crate::utils::aliases::MaybeOwnedPath;
// use crate::utils::aliases::MaybeOwnedString;
// use crate::utils::extensions::OptionExt;

// pub struct DownloadVideoView {
//     progress_bars: ::indicatif::MultiProgress,
//     video_progress_bar: ::indicatif::ProgressBar,
// }

// impl DownloadVideoView {
//     pub fn new() -> Fallible<Self> {
//         static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");
        
//         let progress_bars = ::indicatif::MultiProgress::new();
//         progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

//         let video_progress_bar = progress_bars.add(::indicatif::ProgressBar::new(100)
//             .with_style(PROGRESS_BAR_STYLE.clone()));

//         video_progress_bar.disable_steady_tick();

//         video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", "??MiB", "??MiB/s"), "??:??"));
//         video_progress_bar.set_message(format!("{:>3}%", "??"));

//         Ok(Self { progress_bars, video_progress_bar })
//     }
// }

// #[async_trait]
// impl Activate for DownloadVideoView {
//     async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
//         self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());
//         self.video_progress_bar.tick();

//         Ok(())
//     }

//     async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
//         self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<VideoDownloadEvent> for DownloadVideoView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
//         match event {
//             VideoDownloadEvent::Started(event) => self.update(event).await,
//             VideoDownloadEvent::ProgressUpdated(event) => self.update(event).await,
//             VideoDownloadEvent::Completed(event) => self.update(event).await,
//         }
//     }
// }

// #[async_trait]
// impl Update<VideoDownloadStartedEvent> for DownloadVideoView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadStartedEvent) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         let VideoDownloadStartedEvent { video } = event;

//         let title = video.metadata.title
//             .as_deref()
//             .map_or_else(|| NULL.clone(), |title| title.white().bold());
        
//         self.video_progress_bar.println(format!("Downloading video: {}", title));

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<VideoDownloadProgressUpdatedEvent> for DownloadVideoView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
//         let VideoDownloadProgressUpdatedEvent { id, eta, elapsed, downloaded_bytes, total_bytes, bytes_per_second } = event;

//         self.video_progress_bar.set_position(*percentage as u64);
//         self.video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", size, speed), eta));
//         self.video_progress_bar.set_message(format!("{:>3}%", percentage));

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<VideoDownloadCompletedEvent> for DownloadVideoView {
//     async fn update(self: ::std::sync::Arc<Self>, _: &VideoDownloadCompletedEvent) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");
        
//         self.video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
//         self.video_progress_bar.set_prefix(self.video_progress_bar.prefix().green().to_string());
//         self.video_progress_bar.set_message(self.video_progress_bar.message().green().to_string());

//         self.video_progress_bar.finish();

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<DiagnosticEvent> for DownloadVideoView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{msg}");

//         let DiagnosticEvent { message, level } = &event.payload;

//         let message = match level {
//             DiagnosticLevel::Warning => message.yellow(),
//             DiagnosticLevel::Error => message.red(),
//         };

//         let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
//             .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

//         decoy_progress_bar.finish_with_message(format!("{}", message));

//         Ok(())
//     }
// }

// pub struct DownloadPlaylistView {
//     progress_bars: ::indicatif::MultiProgress,
//     playlist_progress_bar: ::indicatif::ProgressBar,
//     video_progress_bars: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::indicatif::ProgressBar>>>,
// }

// impl DownloadPlaylistView {
//     pub fn new() -> Fallible<Self> {
//         static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");
        
//         let progress_bars = ::indicatif::MultiProgress::new();
//         progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

//         let playlist_progress_bar = progress_bars.add(::indicatif::ProgressBar::no_length()
//             .with_style(PROGRESS_BAR_STYLE.clone()));

//         playlist_progress_bar.set_prefix(format!("{:<27}", ""));
//         playlist_progress_bar.set_message("??/??");

//         let video_progress_bars = ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new()));

//         Ok(Self { progress_bars, playlist_progress_bar, video_progress_bars })
//     }
// }

// #[async_trait]
// impl Activate for DownloadPlaylistView {
//     async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
//         self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());

//         self.playlist_progress_bar.tick();
//         self.video_progress_bars.lock().await
//             .iter()
//             .for_each(|(_, video_progress_bar)| video_progress_bar.tick());

//         Ok(())
//     }

//     async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
//         self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<PlaylistDownloadEvent> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadEvent) -> Fallible<()> {
//         match event {
//             PlaylistDownloadEvent::Started(event) => self.update(event).await,
//             PlaylistDownloadEvent::ProgressUpdated(event) => self.update(event).await,
//             PlaylistDownloadEvent::Completed(event) => self.update(event).await,
//         }
//     }
// }

// #[async_trait]
// impl Update<PlaylistDownloadStartedEvent> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadStartedEvent) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         let PlaylistDownloadStartedEvent { playlist } = &event.payload;

//         let title = playlist.metadata.title
//             .as_deref()
//             .map_or_else(|| NULL.clone(), |title| title.white().bold());

//         let length = playlist.videos
//             .as_deref()
//             .map_or(0, |videos| videos.len());

//         self.playlist_progress_bar.set_length(length as u64);
//         self.playlist_progress_bar.set_message(format!("{}/{}", 0, length));
//         self.playlist_progress_bar.println(format!("Downloading playlist: {}", title));

//         Ok(())
//     }
// }

// #[async_trait]
// impl<'event> Update<EventRef<'event, PlaylistDownloadProgressUpdatedEvent>> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, PlaylistDownloadProgressUpdatedEvent>) -> Fallible<()> {
//         let PlaylistDownloadProgressUpdatedEvent { completed_videos, total_videos, .. } = &event.payload;

//         self.playlist_progress_bar.set_position(*completed as u64);
//         self.playlist_progress_bar.set_message(format!("{}/{}", completed, total));

//         Ok(())
//     }
// }

// #[async_trait]
// impl<'event> Update<EventRef<'event, PlaylistDownloadCompletedEvent>> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, _: &EventRef<'event, PlaylistDownloadCompletedEvent>) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         println!("Completed");

//         static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");

//         self.playlist_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
//         self.playlist_progress_bar.set_prefix(self.playlist_progress_bar.prefix().green().to_string());
//         self.playlist_progress_bar.set_message(self.playlist_progress_bar.message().green().to_string());

//         self.playlist_progress_bar.finish();

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<VideoDownloadEvent> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
//         match &event.payload {
//             VideoDownloadEvent::Started(payload) => self.update(&event.with_payload(payload)).await,
//             VideoDownloadEvent::ProgressUpdated(payload) => self.update(&event.with_payload(payload)).await,
//             VideoDownloadEvent::Completed(payload) => self.update(&event.with_payload(payload)).await,
//         }
//     }
// }

// #[async_trait]
// impl<'event> Update<EventRef<'event, VideoDownloadStartedEvent>> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, VideoDownloadStartedEvent>) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");

//         let video_progress_bar = self.video_progress_bars.lock().await
//             .entry(event.metadata.worker_id.clone())
//             // .entry(event.payload.video.id.clone())
//             .or_insert({
//                 let progress_bar = self.progress_bars.insert_before(&self.playlist_progress_bar, ::indicatif::ProgressBar::new(100));

//                 progress_bar.disable_steady_tick();

//                 progress_bar
//             })
//             .clone();

//         let title = event.payload.video.metadata.title
//             .as_deref()
//             .map_or_else(|| NULL.clone(), |title| title.white().bold());

//         video_progress_bar.set_style(PROGRESS_BAR_STYLE.clone());
        
//         video_progress_bar.set_position(0);
//         video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", "??MiB", "??MiB/s"), "??:??"));
//         video_progress_bar.set_message(format!("{:>3}%  {}", "??", title));

//         Ok(())
//     }
// }

// #[async_trait]
// impl<'event> Update<EventRef<'event, VideoDownloadProgressUpdatedEvent>> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, VideoDownloadProgressUpdatedEvent>) -> Fallible<()> {
//         static REGEX: ::once_cell::sync::Lazy<::regex::Regex> = lazy_regex!(r"^\s*(\d{1,3}|\?{2})%  ");

//         let VideoDownloadProgressUpdatedEvent { percentage, size, speed, eta } = event.payload;

//         let video_progress_bar = self.video_progress_bars.lock().await
//             .get(&event.metadata.worker_id)
//             .some()?
//             .clone();

//         video_progress_bar.set_position(*percentage as u64);
//         video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", size, speed), eta));
//         video_progress_bar.set_message(REGEX.replace(&video_progress_bar.message(), format!("{:>3}%  ", percentage)).into_owned());

//         Ok(())
//     }
// }

// #[async_trait]
// impl<'event> Update<EventRef<'event, VideoDownloadCompletedEvent>> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &EventRef<'event, VideoDownloadCompletedEvent>) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");

//         let video_progress_bar = self.video_progress_bars.lock().await
//             .get(&event.metadata.worker_id)
//             .some()?
//             .clone();
        
//         video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
//         video_progress_bar.set_prefix(video_progress_bar.prefix().green().to_string());
//         video_progress_bar.set_message(video_progress_bar.message().green().to_string());

//         video_progress_bar.finish();

//         Ok(())
//     }
// }

// #[async_trait]
// impl Update<DiagnosticEvent> for DownloadPlaylistView {
//     async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
//         use ::colored::Colorize as _;

//         static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{msg}");

//         let DiagnosticEvent { message, level } = &event.payload;

//         let message = match level {
//             DiagnosticLevel::Warning => message.yellow(),
//             DiagnosticLevel::Error => message.red(),
//         };

//         let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
//             .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

//         decoy_progress_bar.finish_with_message(format!("{}", message));

//         Ok(())
//     }
// }

// static NULL: ::once_cell::sync::Lazy<::colored::ColoredString> = lazy_color!("N/A".yellow().bold());
