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
//     let url = "https://youtu.be/ELj1yXR12bE"; // audio
//     // let url = "https://youtube.com/playlist?list=PL8LPRmXba35e4wJtB2i69ggoNwTU5nywg&si=4QC-oJJajwcaMzUx"; // playlist
//     let output = Command::new("yt-dlp")
//         // .arg("-j")
//         // .arg("--flat-playlist")
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

/*
Necessary tasks:
- Print version of yt-dlp
- Getting metadata of playlist & 
*/


// use std::process::{Command, Stdio};
// use std::io::{BufRead, BufReader};
// use regex::Regex;
// use indicatif::{ProgressBar, ProgressStyle};

// fn main() -> std::io::Result<()> {
//     let url = "https://youtu.be/ELj1yXR12bE";

//     let mut child = Command::new("yt-dlp")
//         .args([
//             "--newline",
//             "--no-warnings",
//             "-f", "bestaudio",
//             url,
//         ])
//         .stdout(Stdio::piped())
//         .spawn()?;

//     let stdout = child.stdout.take().expect("No stdout");
//     let reader = BufReader::new(stdout);

//     let progress_re = Regex::new(
//         r"\[download\]\s+([\d.]+)% of\s+([\d.]+)([KMG]iB) at\s+([\d.]+[KMG]iB/s) ETA ([\d:]+)"
//     ).unwrap();

//     let pb = ProgressBar::new(100); // Set as percent scale first
//     pb.set_style(
//         ProgressStyle::default_bar()
//             .template("{msg} [{bar:40.cyan/blue}] {pos:.1}%/{len:.0}% ({bytes_total}) {wide_msg}")
//             .unwrap()
//             .progress_chars("##-"),
//     );
//     pb.set_message("Downloading");

//     for line in reader.lines() {
//         let line = line?;
//         println!("{}", line);

//         if let Some(cap) = progress_re.captures(&line) {
//             let percent: f64 = cap[1].parse().unwrap();
//             let total_size = format!("{}{}", &cap[2], &cap[3]);
//             let speed = &cap[4];
//             let eta = &cap[5];

//             pb.set_position(percent as u64);
//             pb.set_message(format!("Downloading @ {}", speed));
//             pb.set_message(format!("ETA: {} | Size: {}", eta, total_size));
//         }
//     }

//     pb.finish_with_message("Download complete");
//     child.wait()?;
//     Ok(())
// }

// yt-dlp "https://youtu.be/ELj1yXR12bE" -q -f bestaudio --newline --progress --progress-template "[downloading]%(progress._percent_str)s;%(progress._eta_str)s;%(progress._total_bytes_str)s;%(progress._speed_str)s" --exec "echo [completed]%(filepath)s;%(id)s;%(title)s;%(album)s;%(artist)s;%(genre)s"


use regex::Regex;

fn parse_multivalued(s: &str) -> Vec<String> {
    if s.trim() == "NA" {
        vec![]
    } else {
        s.split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    }
}

fn main() {
    let inputs = [
        "[downloading] 70.5%;00:00;   5.68MiB;   3.45MiB/s",
        "[downloading]100.0%;NA;   5.68MiB;3.39MiB/s",
        "[completed]D:\\root\\dev\\rs\\walkman\\Something I Can Never Have [ELj1yXR12bE].webm;ELj1yXR12bE;Something I Can Never Have;Pretty Hate Machine;Nine Inch Nails;NA",
        "[completed]/music/Moby - Porcelain [abc123].webm;abc123;Porcelain;Play;Moby, Artist B;Electronic, Ambient",
    ];

    let downloading_regex = Regex::new(
        r"\[downloading\](?P<percent>\d+)\.\d+%;(?P<eta>[^;]+);(?P<size>[^;]+);(?P<speed>[^\r\n]+)"
    ).unwrap();

    let completed_regex = Regex::new(
        r"\[completed\](?P<filepath>[^;]+);(?P<id>[^;]+);(?P<title>[^;]+);(?P<album>[^;]+);(?P<artist>[^;]+);(?P<genre>[^\r\n]+)"
    ).unwrap();

    for line in &inputs {
        if let Some(cap) = downloading_regex.captures(line) {
            println!("Downloading:");
            println!("  Percent: {}%", &cap["percent"]);
            println!("  ETA    : {}", &cap["eta"]);
            println!("  Size   : {}", &cap["size"].trim());
            println!("  Speed  : {}", &cap["speed"].trim());
        } else if let Some(cap) = completed_regex.captures(line) {
            let artists = parse_multivalued(&cap["artist"]);
            let genres = parse_multivalued(&cap["genre"]);

            println!("Completed:");
            println!("  File   : {}", &cap["filepath"]);
            println!("  ID     : {}", &cap["id"]);
            println!("  Title  : {}", &cap["title"]);
            println!("  Album  : {}", &cap["album"]);
            println!("  Artists: {:?}", artists);
            println!("  Genres : {:?}", genres);
        } else {
            println!("Unmatched: {}", line);
        }
        println!("---");
    }
}
