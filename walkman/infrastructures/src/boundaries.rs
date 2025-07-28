use ::async_trait::async_trait;
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

use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::extensions::OptionExt;

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
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");
        
        let progress_bars = ::indicatif::MultiProgress::new();
        progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        let video_progress_bar = progress_bars.add(::indicatif::ProgressBar::new(100)
            .with_style(PROGRESS_BAR_STYLE.clone()));

        video_progress_bar.disable_steady_tick();

        video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", "??MiB", "??MiB/s"), "??:??"));
        video_progress_bar.set_message(format!("{:>3}%", "??"));

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

        let title = video.metadata.title
            .as_deref()
            .map_or_else(|| NULL.clone(), |title| title.white().bold());
        
        self.video_progress_bar.println(format!("Downloading video: {}", title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEvent { eta, downloaded_bytes, total_bytes, bytes_per_second, .. } = event;

        let percentage = *downloaded_bytes as f64 / *total_bytes as f64 * 100.0;
        let eta = ::humantime::format_duration(*eta);
        let downloaded_bytes = ::bytesize::ByteSize::b(*downloaded_bytes);
        let speed = ::bytesize::ByteSize::b(*bytes_per_second);

        self.video_progress_bar.set_position(percentage as u64);
        self.video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", downloaded_bytes, speed), eta));
        self.video_progress_bar.set_message(format!("{:>3}%", percentage));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, _: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");
        
        self.video_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.video_progress_bar.set_prefix(self.video_progress_bar.prefix().green().to_string());
        self.video_progress_bar.set_message(self.video_progress_bar.message().green().to_string());

        self.video_progress_bar.finish();

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for DownloadVideoView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{msg}");

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

pub struct DownloadPlaylistView {
    progress_bars: ::indicatif::MultiProgress,
    playlist_progress_bar: ::indicatif::ProgressBar,
    video_progress_bars: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::indicatif::ProgressBar>>>,
}

impl DownloadPlaylistView {
    pub fn new() -> Fallible<Self> {
        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");
        
        let progress_bars = ::indicatif::MultiProgress::new();
        progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::hidden());

        let playlist_progress_bar = progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(PROGRESS_BAR_STYLE.clone()));

        playlist_progress_bar.set_prefix(format!("{:<27}", ""));
        playlist_progress_bar.set_message("??/??");

        let video_progress_bars = ::std::sync::Arc::new(::tokio::sync::Mutex::new(::std::collections::HashMap::new()));

        Ok(Self { progress_bars, playlist_progress_bar, video_progress_bars })
    }
}

#[async_trait]
impl Activate for DownloadPlaylistView {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        self.progress_bars.set_draw_target(::indicatif::ProgressDrawTarget::stderr());

        self.playlist_progress_bar.tick();
        self.video_progress_bars.lock().await
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

        let title = playlist.metadata.title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| NULL.clone());

        let length = playlist.videos
            .as_deref()
            .map(|videos| videos.len())
            .unwrap_or_default();

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

        self.playlist_progress_bar.set_position(*completed_videos as u64);
        self.playlist_progress_bar.set_message(format!("{}/{}", completed_videos, total_videos));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadCompletedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, _: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        println!("Completed");

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");

        self.playlist_progress_bar.set_style(PROGRESS_BAR_FINISH_STYLE.clone());
        self.playlist_progress_bar.set_prefix(self.playlist_progress_bar.prefix().green().to_string());
        self.playlist_progress_bar.set_message(self.playlist_progress_bar.message().green().to_string());

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

        static PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50} {msg}");

        let video_progress_bar = self.video_progress_bars.lock().await
            .entry(event.video.id.clone())
            .or_insert({
                let progress_bar = self.progress_bars.insert_before(&self.playlist_progress_bar, ::indicatif::ProgressBar::new(100));

                progress_bar.disable_steady_tick();

                progress_bar
            })
            .clone();

        let title = event.video.metadata.title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| NULL.clone());

        video_progress_bar.set_style(PROGRESS_BAR_STYLE.clone());
        
        video_progress_bar.set_position(0);
        video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", "??MiB", "??MiB/s"), "??:??"));
        video_progress_bar.set_message(format!("{:>3}%  {}", "??", title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let VideoDownloadProgressUpdatedEvent { id, eta, downloaded_bytes, total_bytes, bytes_per_second, .. } = event;

        let video_progress_bar = self.video_progress_bars.lock().await
            .get(id)
            .ok()?
            .clone();

        let percentage = *downloaded_bytes as f64 / *total_bytes as f64 * 100.0;
        let eta = ::humantime::format_duration(*eta);
        let downloaded_bytes = ::bytesize::ByteSize::b(*downloaded_bytes);
        let speed = ::bytesize::ByteSize::b(*bytes_per_second);

        let message = video_progress_bar.message();
        let idx = message.char_indices()
            .nth(3)
            .map(|(idx, _)| idx)
            .ok()?;
        let message = format!("{:>3}{}", percentage as u64, &message[idx..]);

        video_progress_bar.set_position(percentage as u64);
        video_progress_bar.set_prefix(format!("{:<21} {:4}", format!("{} @ {}", downloaded_bytes, speed), eta));
        video_progress_bar.set_message(message);

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for DownloadPlaylistView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        static PROGRESS_BAR_FINISH_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{prefix} {bar:50.green} {msg}");

        let video_progress_bar = self.video_progress_bars.lock().await
            .get(&event.video.id)
            .ok()?
            .clone();
        
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

        static DECOY_PROGRESS_BAR_STYLE: ::once_cell::sync::Lazy<::indicatif::ProgressStyle> = lazy_progress_style!("{msg}");

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars.add(::indicatif::ProgressBar::no_length()
            .with_style(DECOY_PROGRESS_BAR_STYLE.clone()));

        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

static NULL: ::once_cell::sync::Lazy<::colored::ColoredString> = lazy_color!("N/A".yellow().bold());
