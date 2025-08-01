use ::async_trait::async_trait;
use domain::ChannelId;
use domain::PlaylistId;
use domain::VideoId;
use ::use_cases::boundaries::Activate;
use ::use_cases::boundaries::Update;
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
use ::futures::prelude::*;

use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::extensions::OptionExt;
use crate::utils::extensions::EntryExt;

macro_rules! lazy_progress_style {
    ($template:expr) => {
        ::once_cell::sync::Lazy::new(|| ::indicatif::ProgressStyle::with_template($template).unwrap())
    };
}

macro_rules! lazy_color {
    ($color:expr) => {
        ::once_cell::sync::Lazy::new(|| {
            use ::colored::Colorize as _;

            $color
        })
    };
}

pub struct DownloadVideoView {
    progress_bars: ::indicatif::MultiProgress,
    video_progress_bar: ::indicatif::ProgressBar,
}

impl DownloadVideoView {
    pub fn new() -> Fallible<Self> {
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50} {msg}");

        let progress_bars = ::indicatif::MultiProgress::new();
        progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        let video_progress_bar =
            progress_bars.add(::indicatif::ProgressBar::new(100).with_style(PROGRESS_BAR_STYLE.clone()));

        video_progress_bar.disable_steady_tick();

        let percentage = FormattedUninitPercentage;
        let downloaded_bytes = FormattedUninitBytes;
        let speed = FormattedUninitBytesPerSecond;
        let eta = FormattedUninitDuration;

        video_progress_bar.set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(format!("{}", percentage));

        Ok(Self { progress_bars, video_progress_bar })
    }
}

#[async_trait]
impl Activate for DownloadVideoView {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());
        self.video_progress_bar.tick();

        Ok(())
    }

    async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
        match event {
            VideoDownloadEvent::Started(event) => self.update(event).await,
            VideoDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            VideoDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<VideoDownloadStartedEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let VideoDownloadStartedEvent { video } = event;

        let title = video
            .metadata
            .title
            .as_deref()
            .map_or_else(|| NULL.clone(), |title| title.white().bold());

        self.video_progress_bar.println(format!("Downloading video: {}", title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEvent {
            eta,
            downloaded_bytes,
            total_bytes,
            bytes_per_second,
            ..
        } = event;

        let percentage = *downloaded_bytes as f64 / *total_bytes as f64 * 100.0;
        let percentage = FormattedPercentage(percentage as u64);
        let eta = FormattedDuration(*eta);
        let downloaded_bytes = FormattedBytes(*downloaded_bytes);
        let speed = FormattedBytesPerSecond(*bytes_per_second);

        self.video_progress_bar.set_position(*percentage);
        self.video_progress_bar
            .set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        self.video_progress_bar.set_message(format!("{}", percentage));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, _: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50.green} {msg}");

        self.video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.video_progress_bar
            .set_prefix(self.video_progress_bar.prefix().green().to_string());
        self.video_progress_bar
            .set_message(self.video_progress_bar.message().green().to_string());

        self.video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{msg}");

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self
            .progress_bars
            .add(::indicatif::ProgressBar::no_length().with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

pub struct DownloadPlaylistView {
    progress_bars: ::indicatif::MultiProgress,
    playlist_progress_bar: ::indicatif::ProgressBar,
    video_progress_bars:
        ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::indicatif::ProgressBar>>>,
}

impl DownloadPlaylistView {
    pub fn new() -> Fallible<Self> {
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50} {msg}");

        let progress_bars = ::indicatif::MultiProgress::new();
        progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        let playlist_progress_bar =
            progress_bars.add(::indicatif::ProgressBar::no_length().with_style(PROGRESS_BAR_STYLE.clone()));

        playlist_progress_bar.set_prefix(format!("{:<33}", ""));
        playlist_progress_bar.set_message("??/??");

        let video_progress_bars = ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new()));

        Ok(Self {
            progress_bars,
            playlist_progress_bar,
            video_progress_bars,
        })
    }
}

#[async_trait]
impl Activate for DownloadPlaylistView {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());

        self.playlist_progress_bar.tick();
        self.video_progress_bars
            .lock()
            .await
            .iter()
            .for_each(|(_, video_progress_bar)| video_progress_bar.tick());

        Ok(())
    }

    async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadEvent) -> Fallible<()> {
        match event {
            PlaylistDownloadEvent::Started(event) => self.update(event).await,
            PlaylistDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            PlaylistDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<PlaylistDownloadStartedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let PlaylistDownloadStartedEvent { playlist } = event;

        let title = playlist
            .metadata
            .title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| NULL.clone());

        let length = playlist.videos.as_deref().map(|videos| videos.len()).unwrap_or_default();

        self.playlist_progress_bar.set_length(length as u64);
        self.playlist_progress_bar.set_message(format!("{}/{}", 0, length));
        self.playlist_progress_bar.println(format!("Downloading playlist: {}", title));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadProgressUpdatedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadProgressUpdatedEvent) -> Fallible<()> {
        let PlaylistDownloadProgressUpdatedEvent { completed_videos, total_videos, .. } = event;

        self.playlist_progress_bar.set_position(*completed_videos);
        self.playlist_progress_bar
            .set_message(format!("{}/{}", completed_videos, total_videos));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadCompletedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, _: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50.green} {msg}");

        self.playlist_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.playlist_progress_bar
            .set_prefix(self.playlist_progress_bar.prefix().green().to_string());
        self.playlist_progress_bar
            .set_message(self.playlist_progress_bar.message().green().to_string());

        self.playlist_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
        match event {
            VideoDownloadEvent::Started(event) => self.update(event).await,
            VideoDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            VideoDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<VideoDownloadStartedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50} {msg}");

        let video_progress_bar = self
            .video_progress_bars
            .lock()
            .await
            .entry(event.video.id.clone())
            .or_insert({
                let progress_bar = self
                    .progress_bars
                    .insert_before(&self.playlist_progress_bar, ::indicatif::ProgressBar::new(100));

                progress_bar.disable_steady_tick();

                progress_bar
            })
            .clone();

        let title = event
            .video
            .metadata
            .title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| NULL.clone());

        video_progress_bar.set_style(PROGRESS_BAR_STYLE.clone());

        let percentage = FormattedUninitPercentage;
        let downloaded_bytes = FormattedUninitBytes;
        let speed = FormattedUninitBytesPerSecond;
        let eta = FormattedUninitDuration;

        video_progress_bar.set_position(0);
        video_progress_bar.set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(format!("{}  {}", percentage, title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEvent {
            video_id,
            eta,
            downloaded_bytes,
            total_bytes,
            bytes_per_second,
            ..
        } = event;

        let video_progress_bar = self.video_progress_bars.lock().await.get(video_id).ok()?.clone();

        let percentage = *downloaded_bytes as f64 / *total_bytes as f64 * 100.0;
        let percentage = FormattedPercentage(percentage as u64);
        let eta = FormattedDuration(*eta);
        let downloaded_bytes = FormattedBytes(*downloaded_bytes);
        let speed = FormattedBytesPerSecond(*bytes_per_second);

        let message = video_progress_bar.message();
        let idx = message.char_indices().nth(4).map(|(idx, _)| idx).ok()?;
        let message = format!("{:>3}{}", percentage, &message[idx..]);

        video_progress_bar.set_position(*percentage);
        video_progress_bar.set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(message);

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{prefix} {bar:50.green} {msg}");

        let video_progress_bar = self.video_progress_bars.lock().await.get(&event.video.id).ok()?.clone();

        video_progress_bar.set_length(100);
        video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        video_progress_bar.set_prefix(video_progress_bar.prefix().green().to_string());
        video_progress_bar.set_message(video_progress_bar.message().green().to_string());

        video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{msg}");

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self
            .progress_bars
            .add(::indicatif::ProgressBar::no_length().with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

pub struct AggregateView {
    progress_bars: ::indicatif::MultiProgress,

    video_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<VideoId, ::indicatif::ProgressBar>>>,
    playlist_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<PlaylistId, ::indicatif::ProgressBar>>>,
    channel_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<ChannelId, ::indicatif::ProgressBar>>>,

    playlist_ids_by_video_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<VideoId, PlaylistId>>>,
    channel_ids_by_video_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<VideoId, ChannelId>>>,
    channel_ids_by_playlist_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<PlaylistId, ChannelId>>>,
}

impl AggregateView {
    pub fn new() -> Self {
        Self {
            progress_bars: ::indicatif::MultiProgress::new(),

            video_progress_bars_by_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),
            playlist_progress_bars_by_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),
            channel_progress_bars_by_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),

            playlist_ids_by_video_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),
            channel_ids_by_video_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),
            channel_ids_by_playlist_ids: ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl Activate for AggregateView {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());

        self.video_progress_bars_by_ids.lock().await
            .values()
            .for_each(|progress_bar| progress_bar.tick());

        self.playlist_progress_bars_by_ids.lock().await
            .values()
            .for_each(|progress_bar| progress_bar.tick());

        self.channel_progress_bars_by_ids.lock().await
            .values()
            .for_each(|progress_bar| progress_bar.tick());

        Ok(())
    }

    async fn deactivate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadEvent) -> Fallible<()> {
        match event {
            VideoDownloadEvent::Started(event) => self.update(event).await,
            VideoDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            VideoDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<VideoDownloadStartedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let mut video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars
            .entry(event.video.id.clone())
            .or_insert_with_future(|| async {
                if let Some(channel_id) = self.channel_ids_by_video_ids.lock().await.get(&event.video.id) {
                    let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
                    let channel_progress_bar = channel_progress_bars.get(channel_id).ok().unwrap();
                    self.progress_bars.insert_after(&channel_progress_bar, ::indicatif::ProgressBar::new(100))  
                } else if let Some(playlist_id) = self.playlist_ids_by_video_ids.lock().await.get(&event.video.id) {
                    let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
                    let playlist_progress_bar = playlist_progress_bars.get(playlist_id).ok().unwrap();
                    self.progress_bars.insert_after(&playlist_progress_bar, ::indicatif::ProgressBar::new(100))
                } else {
                    self.progress_bars.add(::indicatif::ProgressBar::new(100))
                }
            })
            .await;

        let title = event.video.metadata.title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| "N/A".yellow().bold());

        video_progress_bar.disable_steady_tick();
        video_progress_bar.set_style(::indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")?
            .progress_chars("#>-"));

        let percentage = FormattedUninitPercentage;
        let downloaded_bytes = FormattedUninitBytes;
        let speed = FormattedUninitBytesPerSecond;
        let eta = FormattedUninitDuration;

        video_progress_bar.set_position(0);
        video_progress_bar.set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(format!("{}  {}", percentage, title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars.get(&event.video_id).ok()?;

        let percentage = *&event.downloaded_bytes as f64 / *&event.total_bytes as f64 * 100.0;
        let percentage = FormattedPercentage(percentage as u64);
        let eta = FormattedDuration(*&event.eta);
        let downloaded_bytes = FormattedBytes(*&event.downloaded_bytes);
        let speed = FormattedBytesPerSecond(*&event.bytes_per_second);

        let message = video_progress_bar.message();
        let idx = message.char_indices().nth(4).map(|(idx, _)| idx).ok()?;
        let message = format!("{:>3}{}", percentage, &message[idx..]);

        video_progress_bar.set_position(*percentage);
        video_progress_bar.set_prefix(format!("{:<24} {}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(message);

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars.get(&event.video.id).ok()?;

        video_progress_bar.set_position(100);
        // video_progress_bar.set_style(::indicatif::ProgressStyle::with_template("{prefix} {bar:50.green} {msg}")?
        //     .progress_chars("#>-"));
        video_progress_bar.set_prefix(video_progress_bar.prefix().color(GRAY).to_string());
        video_progress_bar.set_message(video_progress_bar.message().color(GRAY).to_string());

        video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadEvent) -> Fallible<()> {
        match event {
            PlaylistDownloadEvent::Started(event) => self.update(event).await,
            PlaylistDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            PlaylistDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<PlaylistDownloadStartedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let mut playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars
            .entry(event.playlist.id.clone())
            .or_insert_with_future(|| async {
                if let Some(channel_id) = self.channel_ids_by_playlist_ids.lock().await.get(&event.playlist.id) {
                    let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
                    let channel_progress_bar = channel_progress_bars.get(channel_id).ok().unwrap();
                    self.progress_bars.insert_after(&channel_progress_bar, ::indicatif::ProgressBar::new(100))
                } else {
                    self.progress_bars.add(::indicatif::ProgressBar::new(100))
                }
            })
            .await;

        ::futures::stream::iter(
            event.playlist.videos
                .as_deref()
                .map(|videos| videos.iter())
                .into_iter()
                .flatten()
        )
            .for_each(|video| async { self.playlist_ids_by_video_ids.lock().await.insert(video.id.clone(), event.playlist.id.clone()); })
            .await;

        let title = event.playlist.metadata.title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| "N/A".yellow().bold());
        let length = event.playlist.videos.as_deref().map(|videos| videos.len()).unwrap_or_default();

        playlist_progress_bar.disable_steady_tick();
        playlist_progress_bar.set_style(::indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")?
            .progress_chars("##-"));
        playlist_progress_bar.set_length(length as u64);
        playlist_progress_bar.set_message(format!("{}/{}", 0, length));
        playlist_progress_bar.println(format!("Downloading playlist: {}", title));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadProgressUpdatedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadProgressUpdatedEvent) -> Fallible<()> {
        let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars.get(&event.playlist_id).ok().unwrap();

        playlist_progress_bar.set_position(*&event.completed_videos);
        playlist_progress_bar.set_message(format!("{}/{}", &event.completed_videos, &event.total_videos));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadCompletedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars.get(&event.playlist.id).ok().unwrap();

        playlist_progress_bar.set_position(playlist_progress_bar.length().ok()?);
        // playlist_progress_bar.set_style(::indicatif::ProgressStyle::with_template("{prefix} {bar:50.green} {msg}")?
        //     .progress_chars("##-"));
        playlist_progress_bar.set_prefix(playlist_progress_bar.prefix().color(GRAY).to_string());
        playlist_progress_bar.set_message(playlist_progress_bar.message().color(GRAY).to_string());

        playlist_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> =
            lazy_progress_style!("{msg}");

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars
            .add(::indicatif::ProgressBar::no_length());

        decoy_progress_bar.set_style(DECOY_PROGRESS_BAR_STYLE.clone());
        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

static NULL: ::once_cell::sync::Lazy<::colored::ColoredString> = lazy_color!("N/A".yellow().bold());

const GRAY: ::colored::Color = ::colored::Color::TrueColor { r: 150, g: 150, b: 150 };

struct FormattedPercentage(u64);

impl ::std::ops::Deref for FormattedPercentage {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ::std::fmt::Display for FormattedPercentage {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{:>3}%", self.0)
    }
}

struct FormattedUninitPercentage;

impl ::std::fmt::Display for FormattedUninitPercentage {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{:>3}%", "??")
    }
}

struct FormattedDuration(::std::time::Duration);

impl ::std::fmt::Display for FormattedDuration {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        let duration = ::time::Duration::try_from(self.0).unwrap();

        let hours = duration.whole_hours() % 24;
        let minutes = duration.whole_minutes() % 60;
        let seconds = duration.whole_seconds() % 60;

        write!(formatter, "{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

struct FormattedUninitDuration;

impl ::std::fmt::Display for FormattedUninitDuration {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{:02}:{:02}:{:02}", "??", "??", "??")
    }
}

struct FormattedBytes(u64);

impl ::std::fmt::Display for FormattedBytes {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{}", ::bytesize::ByteSize::b(self.0))
    }
}

struct FormattedUninitBytes;

impl ::std::fmt::Display for FormattedUninitBytes {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "??MiB")
    }
}

struct FormattedBytesPerSecond(u64);

impl ::std::fmt::Display for FormattedBytesPerSecond {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{}/s", FormattedBytes(self.0))
    }
}

struct FormattedUninitBytesPerSecond;

impl ::std::fmt::Display for FormattedUninitBytesPerSecond {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(formatter, "{}/s", FormattedUninitBytes)
    }
}
