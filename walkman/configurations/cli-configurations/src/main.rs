use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    // let bar = ProgressBar::new(100);
    // bar.set_style(
    //     ProgressStyle::default_bar()
    //         .template("{msg} [{bar:40.cyan/blue}] {pos}%")
    //         .unwrap(),
    // );

    // bar.set_message("Progress");

    // bar.set_position(51);
    // sleep(Duration::from_millis(500)).await;

    // bar.set_position(12);
    // sleep(Duration::from_millis(500)).await;

    // bar.set_position(38);
    // sleep(Duration::from_millis(500)).await;

    // bar.finish_with_message("Done");

    use std::process::{Command, Stdio};

    // stdout must be configured with `Stdio::piped` in order to use
    // `echo_child.stdout`
    let echo_child = Command::new("echo")
        .arg("Oh no, a tpyo!")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start echo process");

    // Note that `echo_child` is moved here, but we won't be needing
    // `echo_child` anymore
    let echo_out = echo_child.stdout.expect("Failed to open echo stdout");

    let mut sed_child = Command::new("sed")
        .arg("s/tpyo/typo/")
        .stdin(Stdio::from(echo_out))
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start sed process");

    let output = sed_child.wait_with_output().expect("Failed to wait on sed");
    assert_eq!(b"Oh no, a typo!\n", output.stdout.as_slice());
}

