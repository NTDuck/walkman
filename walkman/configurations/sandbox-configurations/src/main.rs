use indicatif::{ProgressBar, ProgressStyle};
use std::{thread, time::Duration};

fn main() {
    let pb = ProgressBar::new(100);
    let normal_style = ProgressStyle::with_template("{bar:40.cyan/blue} {pos:>3}/{len}")
        .unwrap();
    let blink_style = ProgressStyle::with_template("{bar:40.green/white} {pos:>3}/{len}")
        .unwrap();

    for i in 0..=100 {
        pb.set_style(blink_style.clone()); // Blink color
        pb.set_position(i);
        thread::sleep(Duration::from_millis(1));
        pb.set_style(normal_style.clone()); // Revert
        pb.tick();
        thread::sleep(Duration::from_millis(100)); // Optional: makes blink noticeable
    }

    pb.finish_with_message("Done");
}
