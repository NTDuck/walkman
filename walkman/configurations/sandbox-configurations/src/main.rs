use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{thread, time::Duration};

fn main() {
    let m = MultiProgress::new();

    // Real progress bar
    let pb = m.add(ProgressBar::new(100));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos:>3}/{len:3} {msg}")
            .unwrap(),
    );

    // Decoy "print-below" progress bar
    let decoy = m.add(ProgressBar::new_spinner());
    decoy.set_style(
        ProgressStyle::default_spinner()
            .template("{msg}")
            .unwrap(),
    );
    decoy.set_message("Download will finish soon...");
    decoy.finish();

    // Spawn actual work
    let handle = thread::spawn({
        let pb = pb.clone();
        move || {
            for i in 0..=100 {
                pb.set_position(i);
                thread::sleep(Duration::from_millis(20));
            }
            pb.finish_with_message("Done");
        }
    });

    let _ = handle.join();

    // Decoy stays visible by not being cleared or finished
    // decoy.set_message("✅ Download complete.");
    // Optionally, stop the spinner animation
    // decoy.enable_steady_tick(Duration::from_millis(0));
}
