use std::thread;
use std::time::Duration;
use std::{cmp::min, fmt::Write};

use colored::{Color, Colorize};
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};

fn main() {
    let mut downloaded = 0;
    let total_size = 231231231;

    let mg = MultiProgress::new();

    for i in 0..255 {
        let pb = mg.add(ProgressBar::new(total_size));
        pb.set_style(ProgressStyle::default_bar().template("{msg}").unwrap());
        pb.set_message(format!("msg {}", i).color(Color::TrueColor { r: i, g: i, b: i }).to_string());
        thread::sleep(Duration::from_millis(12));
        pb.finish();
    }
}
