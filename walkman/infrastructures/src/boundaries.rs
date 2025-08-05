use ::async_trait::async_trait;
use ::use_cases::boundaries::Activate;
use ::use_cases::boundaries::Update;
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
use ::futures::prelude::*;

use crate::utils::aliases::Fallible;
use crate::utils::aliases::MaybeOwnedString;
use crate::utils::extensions::OptionExt;
use crate::utils::extensions::EntryExt;

#[derive(::bon::Builder)]
#[builder(on(_, into))]
pub struct AggregateView {
    #[builder(skip)]
    progress_bars: ::indicatif::MultiProgress,

    #[builder(skip)]
    video_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::std::sync::Arc<VideoProgressBar>>>>,
    #[builder(skip)]
    playlist_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::std::sync::Arc<PlaylistProgressBar>>>>,
    #[builder(skip)]
    channel_progress_bars_by_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, ::std::sync::Arc<ChannelProgressBar>>>>,

    #[builder(skip)]
    playlist_ids_by_video_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, MaybeOwnedString>>>,
    #[builder(skip)]
    channel_ids_by_video_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, MaybeOwnedString>>>,
    #[builder(skip)]
    channel_ids_by_playlist_ids: ::std::sync::Arc<::tokio::sync::Mutex<::std::collections::HashMap<MaybeOwnedString, MaybeOwnedString>>>,
}

#[async_trait]
impl Activate for AggregateView {
    async fn activate(self: ::std::sync::Arc<Self>) -> Fallible<()> {
        ::tracing::debug!("Activated `AggregateView`");

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
        ::tracing::debug!("Deactivated `AggregateView`");

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
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let mut video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars
            .entry(event.video.id.clone())
            .or_insert_with_future(|| async {
                let progress_bar = if let Some(channel_id) = self.channel_ids_by_video_ids.lock().await.get(&event.video.id) {
                    let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
                    let channel_progress_bar = channel_progress_bars.get(channel_id).ok().unwrap();
                    self.progress_bars.insert_after(&channel_progress_bar, VideoProgressBar::default().into())
                } else if let Some(playlist_id) = self.playlist_ids_by_video_ids.lock().await.get(&event.video.id) {
                    let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
                    let playlist_progress_bar = playlist_progress_bars.get(playlist_id).ok().unwrap();
                    self.progress_bars.insert_after(&playlist_progress_bar, VideoProgressBar::default().into())
                } else {
                    self.progress_bars.add(VideoProgressBar::default().into())
                };
                
                ::std::sync::Arc::new(progress_bar.into())
            })
            .await;

        ::std::sync::Arc::clone(video_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars.get(&event.video_id).ok()?;

        ::std::sync::Arc::clone(video_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let video_progress_bars = self.video_progress_bars_by_ids.lock().await;
        let video_progress_bar = video_progress_bars.get(&event.video.id).ok()?;

        ::std::sync::Arc::clone(video_progress_bar).update(event).await?;

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
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let mut playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars
            .entry(event.playlist.id.clone())
            .or_insert_with_future(|| async {
                let progress_bar = if let Some(channel_id) = self.channel_ids_by_playlist_ids.lock().await.get(&event.playlist.id) {
                    let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
                    let channel_progress_bar = channel_progress_bars.get(channel_id).ok().unwrap();
                    self.progress_bars.insert_after(&channel_progress_bar, PlaylistProgressBar::default().into())
                } else {
                    self.progress_bars.add(PlaylistProgressBar::default().into())
                };

                ::std::sync::Arc::new(progress_bar.into())
            })
            .await;

        ::futures::stream::iter(event.playlist.videos
            .as_deref()
            .into_iter()
            .flatten())
            .for_each(|video| async { self.playlist_ids_by_video_ids.lock().await.insert(video.id.clone(), event.playlist.id.clone()); })
            .await;

        ::std::sync::Arc::clone(playlist_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadProgressUpdatedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadProgressUpdatedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars.get(&event.playlist_id).ok()?;

        ::std::sync::Arc::clone(playlist_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadCompletedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let playlist_progress_bars = self.playlist_progress_bars_by_ids.lock().await;
        let playlist_progress_bar = playlist_progress_bars.get(&event.playlist.id).ok()?;

        ::std::sync::Arc::clone(playlist_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<ChannelDownloadEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadEvent) -> Fallible<()> {
        match event {
            ChannelDownloadEvent::Started(event) => self.update(event).await,
            ChannelDownloadEvent::ProgressUpdated(event) => self.update(event).await,
            ChannelDownloadEvent::Completed(event) => self.update(event).await,
        }
    }
}

#[async_trait]
impl Update<ChannelDownloadStartedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadStartedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let mut channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
        let channel_progress_bar = channel_progress_bars
            .entry(event.channel.id.clone())
            .or_insert_with(|| ::std::sync::Arc::new(self.progress_bars.add(ChannelProgressBar::default().into()).into()));

        ::tokio::join!(
            ::futures::stream::iter(event.channel.videos
                .as_deref()
                .into_iter()
                .flatten())
                .for_each(|videos| async { self.channel_ids_by_video_ids.lock().await.insert(videos.id.clone(), event.channel.id.clone()); })
                .boxed(),
            ::futures::stream::iter(event.channel.playlists
                .as_deref()
                .into_iter()
                .flatten())
                .for_each(|playlist| async { self.channel_ids_by_playlist_ids.lock().await.insert(playlist.id.clone(), event.channel.id.clone()); })
                .boxed(),
        );

        ::std::sync::Arc::clone(channel_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<ChannelDownloadProgressUpdatedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadProgressUpdatedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
        let channel_progress_bar = channel_progress_bars.get(&event.channel_id).ok()?;

        ::std::sync::Arc::clone(channel_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<ChannelDownloadCompletedEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadCompletedEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        let channel_progress_bars = self.channel_progress_bars_by_ids.lock().await;
        let channel_progress_bar = channel_progress_bars.get(&event.channel.id).ok()?;

        ::std::sync::Arc::clone(channel_progress_bar).update(event).await?;

        Ok(())
    }
}

#[async_trait]
impl Update<DiagnosticEvent> for AggregateView {
    async fn update(self: ::std::sync::Arc<Self>, event: &DiagnosticEvent) -> Fallible<()> {
        ::tracing::debug!("Received (OB) event `{:?}`", event);

        use ::colored::Colorize as _;

        let DiagnosticEvent { message, level } = event;

        let message = match level {
            DiagnosticLevel::Warning => message.yellow(),
            DiagnosticLevel::Error => message.red(),
        };

        let decoy_progress_bar = self.progress_bars
            .add(::indicatif::ProgressBar::no_length());

        decoy_progress_bar.set_style(::indicatif::ProgressStyle::with_template("{msg}")?);
        decoy_progress_bar.finish_with_message(format!("{}", message));

        Ok(())
    }
}

struct VideoProgressBar(::indicatif::ProgressBar);

impl Default for VideoProgressBar {
    fn default() -> Self {
        Self(::indicatif::ProgressBar::no_length())
    }
}

impl ::std::ops::Deref for VideoProgressBar {
    type Target = ::indicatif::ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<VideoProgressBar> for ::indicatif::ProgressBar {
    fn from(outer: VideoProgressBar) -> Self {
        outer.0
    }
}

impl From<::indicatif::ProgressBar> for VideoProgressBar {
    fn from(inner: ::indicatif::ProgressBar) -> Self {
        Self(inner)
    }
}

#[async_trait]
impl Update<VideoDownloadStartedEvent> for VideoProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.video.metadata.title
            .as_deref()
            .map(|title| title.white().bold())
            .unwrap_or_else(|| "N/A".yellow().bold());
        
        let (downloaded_bytes, speed, eta) = (FormattedUninitBytes, FormattedUninitBytesPerSecond, FormattedUninitDuration);

        self.disable_steady_tick();

        self.set_length(100);
        self.set_position(0);

        self.set_style(::indicatif::ProgressStyle::with_template("{prefix} {bar:50} {msg}")?
            .progress_chars("#>-"));
        self.set_prefix(format!("[{}]", eta));
        self.set_message(format!("[{} @ {}] {}", downloaded_bytes, speed, title));
        
        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadProgressUpdatedEvent> for VideoProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadProgressUpdatedEvent) -> Fallible<()> {
        let message = self.message();
        let title = message
            .rfind("] ")
            .map(|idx| &message[idx + 2..])
            .ok()?;

        let eta = FormattedDuration(*&event.eta);
        let downloaded_bytes = FormattedBytes(*&event.downloaded_bytes);
        let speed = FormattedBytesPerSecond(*&event.bytes_per_second);

        self.set_position(event.percentage);

        self.set_prefix(format!("[{}]", eta));
        self.set_message(format!("[{} @ {}] {}", downloaded_bytes, speed, title));

        Ok(())
    }
}

#[async_trait]
impl Update<VideoDownloadCompletedEvent> for VideoProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &VideoDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.video.metadata.title
            .as_deref()
            .map(|title| title.bold())
            .unwrap_or_else(|| "N/A".bold());

        let message = self.message();
        let message = message
            .rfind("] ")
            .map(|idx| &message[..=idx + 1])
            .map(|prefix| format!("{}{}", prefix, title))
            .ok()?;

        self.set_length(100);
        self.set_position(100);

        self.set_style(::indicatif::ProgressStyle::with_template(&format!("{{prefix}} {:#<50} {{msg}}", "".gray()))?);
        self.set_prefix(self.prefix().normal().gray().to_string());
        self.set_message(message.normal().gray().to_string());

        self.finish();

        Ok(())
    }
}

struct PlaylistProgressBar(::indicatif::ProgressBar);

impl Default for PlaylistProgressBar {
    fn default() -> Self {
        Self(::indicatif::ProgressBar::no_length())
    }
}

impl ::std::ops::Deref for PlaylistProgressBar {
    type Target = ::indicatif::ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<PlaylistProgressBar> for ::indicatif::ProgressBar {
    fn from(outer: PlaylistProgressBar) -> Self {
        outer.0
    }
}

impl From<::indicatif::ProgressBar> for PlaylistProgressBar {
    fn from(inner: ::indicatif::ProgressBar) -> Self {
        Self(inner)
    }
}

#[async_trait]
impl Update<PlaylistDownloadStartedEvent> for PlaylistProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.playlist.metadata.title
            .as_deref()
            .map(|title| title.bold())
            .unwrap_or_else(|| "N/A".yellow().bold());

        let total_videos = event.playlist.videos.as_deref().map(|videos| videos.len()).unwrap_or_default();

        self.disable_steady_tick();
        
        self.set_length(total_videos as u64);
        self.set_position(0);
        
        self.set_style(::indicatif::ProgressStyle::with_template("{bar:61} {msg}")?
            .progress_chars("##-"));
        self.set_message(format!("[{}/{}] {}", 0, total_videos, title));
        
        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadProgressUpdatedEvent> for PlaylistProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadProgressUpdatedEvent) -> Fallible<()> {
        let message = self.message();
        let title = message
            .rfind("] ")
            .map(|idx| &message[idx + 2..])
            .ok()?;

        self.set_position(event.completed_videos);

        self.set_message(format!("[{}/{}] {}", event.completed_videos, event.total_videos, title));

        Ok(())
    }
}

#[async_trait]
impl Update<PlaylistDownloadCompletedEvent> for PlaylistProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &PlaylistDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.playlist.metadata.title
            .as_deref()
            .map(|title| title.bold())
            .unwrap_or_else(|| "N/A".bold());

        let message = self.message();
        let message = message
            .rfind("] ")
            .map(|idx| &message[..=idx + 1])
            .map(|prefix| format!("{}{}", prefix, title))
            .ok()?;

        self.set_position(self.length().ok()?);

        self.set_style(::indicatif::ProgressStyle::with_template(&format!("{:#<61} {{msg}}", "".gray()))?);
        self.set_message(message.normal().gray().to_string());

        self.finish();

        Ok(())
    }
}

struct ChannelProgressBar(::indicatif::ProgressBar);

impl Default for ChannelProgressBar {
    fn default() -> Self {
        Self(::indicatif::ProgressBar::no_length())
    }
}

impl ::std::ops::Deref for ChannelProgressBar {
    type Target = ::indicatif::ProgressBar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ChannelProgressBar> for ::indicatif::ProgressBar {
    fn from(outer: ChannelProgressBar) -> Self {
        outer.0
    }
}

impl From<::indicatif::ProgressBar> for ChannelProgressBar {
    fn from(inner: ::indicatif::ProgressBar) -> Self {
        Self(inner)
    }
}

#[async_trait]
impl Update<ChannelDownloadStartedEvent> for ChannelProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadStartedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.channel.metadata.title
            .as_deref()
            .map(|title| title.bold())
            .unwrap_or_else(|| "N/A".yellow().bold());

        let total_videos = event.channel.videos.as_deref().map(|videos| videos.len()).unwrap_or_default();
        let total_playlists = event.channel.playlists.as_deref().map(|playlists| playlists.len()).unwrap_or_default();

        self.disable_steady_tick();
        
        self.set_length((total_videos + total_playlists) as u64);
        self.set_position(0);
        
        self.set_style(::indicatif::ProgressStyle::with_template("{bar:61} {msg}")?
            .progress_chars("##-"));
        self.set_message(format!("[{}/{} | {}/{}] {}", 0, total_videos, 0, total_playlists, title));
        
        Ok(())
    }
}

#[async_trait]
impl Update<ChannelDownloadProgressUpdatedEvent> for ChannelProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadProgressUpdatedEvent) -> Fallible<()> {
        let message = self.message();
        let title = message
            .rfind("] ")
            .map(|idx| &message[idx + 2..])
            .ok()?;

        self.set_position(event.completed_videos + event.completed_playlists);

        self.set_message(format!("[{}/{} | {}/{}] {}", event.completed_videos, event.total_videos, event.completed_playlists, event.total_playlists, title));

        Ok(())
    }
}

#[async_trait]
impl Update<ChannelDownloadCompletedEvent> for ChannelProgressBar {
    async fn update(self: ::std::sync::Arc<Self>, event: &ChannelDownloadCompletedEvent) -> Fallible<()> {
        use ::colored::Colorize as _;

        let title = event.channel.metadata.title
            .as_deref()
            .map(|title| title.bold())
            .unwrap_or_else(|| "N/A".bold());

        let message = self.message();
        let message = message
            .rfind("] ")
            .map(|idx| &message[..=idx + 1])
            .map(|prefix| format!("{}{}", prefix, title))
            .ok()?;

        self.set_position(self.length().ok()?);

        self.set_style(::indicatif::ProgressStyle::with_template(&format!("{:#<61} {{msg}}", "".gray()))?);
        self.set_message(message.normal().gray().to_string());

        self.finish();

        Ok(())
    }
}

trait ColorizeExt {
    fn gray(self) -> ::colored::ColoredString
    where
        Self: Sized;
}

impl<T> ColorizeExt for T
where
    T: ::colored::Colorize,
{
    fn gray(self) -> ::colored::ColoredString {
        self.color(::colored::Color::TrueColor { r: 150, g: 150, b: 150 })
    }
}

// struct FormattedPercentage(u64);

// impl ::std::ops::Deref for FormattedPercentage {
//     type Target = u64;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl ::std::fmt::Display for FormattedPercentage {
//     fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
//         write!(formatter, "{:>3}%", self.0)
//     }
// }

// struct FormattedUninitPercentage;

// impl ::std::fmt::Display for FormattedUninitPercentage {
//     fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
//         write!(formatter, "{:>3}%", "??")
//     }
// }

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
