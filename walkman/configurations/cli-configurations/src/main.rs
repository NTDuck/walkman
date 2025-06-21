use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let bar = ProgressBar::new(100);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}%")
            .unwrap(),
    );

    bar.set_message("Progress");

    bar.set_position(51);
    sleep(Duration::from_millis(500)).await;

    bar.set_position(12);
    sleep(Duration::from_millis(500)).await;

    bar.set_position(38);
    sleep(Duration::from_millis(500)).await;

    bar.finish_with_message("Done");
}
