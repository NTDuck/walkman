use async_trait::async_trait;

#[async_trait]
pub trait DownloadVideoInputBoundary {
    async fn apply(&self, model: DownloadVideoRequestModel);
}

pub struct DownloadVideoRequestModel {
    pub url: String,
}

#[async_trait]
pub trait DownloadVideoOutputBoundary: Send + Sync {
    async fn refresh(&self);
    async fn update(&self, snapshot: DownloadVideoProgressSnapshot);
    async fn terminate(&self);
}

pub struct DownloadVideoProgressSnapshot {
    pub percentage: u8,
    pub eta: std::time::Duration,
    pub size: String,
    pub rate: String,
}

#[async_trait]
pub trait DownloadPlaylistInputBoundary {
    async fn apply(&self, model: DownloadPlaylistRequestModel);
}

pub struct DownloadPlaylistRequestModel {
    pub url: String,
}

#[async_trait]
pub trait DownloadPlaylistOutputBoundary: Send + Sync {
    async fn update(&self, snapshot: DownloadPlaylistProgressSnapshot);
    async fn terminate(&self);
}

// TODO consult docs for exact lim
#[derive(Default)]
pub struct DownloadPlaylistProgressSnapshot {
    pub downloaded_videos_count: usize,
    pub total_videos_count: usize,
}
