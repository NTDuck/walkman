// use std::process::{Command, Stdio};
// use std::io::{BufReader, BufRead};
// use serde::Deserialize;

// #[derive(Debug, Deserialize)]
// struct PlaylistEntry {
//     id: String,
//     title: String,
//     url: Option<String>,
// }

// fn main() -> std::io::Result<()> {
//     let url = "https://www.youtube.com/watch?v=4A6Wdy1NDME&list=RD4A6Wdy1NDME"; // audio
//     let url = "https://youtube.com/playlist?list=PL8LPRmXba35e4wJtB2i69ggoNwTU5nywg&si=4QC-oJJajwcaMzUx"; // playlist
//     let output = Command::new("yt-dlp")
//         .arg("-j")
//         .arg("--flat-playlist")
//         .arg(url)
//         .stdout(Stdio::piped())
//         .spawn()?
//         .stdout
//         .expect("Failed to capture stdout");

//     let reader = BufReader::new(output);
//     for line in reader.lines() {
//         let line = line?;
//         let entry: PlaylistEntry = serde_json::from_str(&line).unwrap();
//         println!("{:?}", entry);
//     }

//     Ok(())
// }

// /*
// Necessary tasks:
// - Print version of yt-dlp
// - Getting metadata of playlist & 
// */


use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use regex::Regex;
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> std::io::Result<()> {
    let url = "https://youtu.be/ELj1yXR12bE";

    let mut child = Command::new("yt-dlp")
        .args([
            "--newline",
            "--no-warnings",
            "-f", "bestaudio",
            url,
        ])
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("No stdout");
    let reader = BufReader::new(stdout);

    let progress_re = Regex::new(
        r"\[download\]\s+([\d.]+)% of\s+([\d.]+)([KMG]iB) at\s+([\d.]+[KMG]iB/s) ETA ([\d:]+)"
    ).unwrap();

    let pb = ProgressBar::new(100); // Set as percent scale first
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos:.1}%/{len:.0}% ({bytes_total}) {wide_msg}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Downloading");

    for line in reader.lines() {
        let line = line?;

        if let Some(cap) = progress_re.captures(&line) {
            let percent: f64 = cap[1].parse().unwrap();
            let total_size = format!("{}{}", &cap[2], &cap[3]);
            let speed = &cap[4];
            let eta = &cap[5];

            pb.set_position(percent as u64);
            pb.set_message(format!("Downloading @ {}", speed));
            pb.set_message(format!("ETA: {} | Size: {}", eta, total_size));
        }
    }

    pb.finish_with_message("Download complete");
    child.wait()?;
    Ok(())
}
