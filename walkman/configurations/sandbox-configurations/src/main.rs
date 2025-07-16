// use anyhow::Result;
// use duct::cmd;
// use tokio::task;

// #[tokio::main]
// async fn main() -> Result<()> {
//     let messages = vec!["first", "second", "third"];

//     let handles = messages.into_iter().map(|msg| {
//         let msg = msg.to_string();
//         task::spawn_blocking(move || {
//             // Simulate yt-dlp with echo
//             cmd!("sh", "-c", format!("echo downloading {msg}; sleep 1; echo done {msg}"))
//                 .stderr_to_stdout()
//                 .stdout_capture()
//                 .read()
//         })
//     });

//     let results = futures::future::try_join_all(handles).await?;

//     for (i, output) in results.into_iter().enumerate() {
//         println!("=== Output {} ===\n{}", i + 1, output?);
//     }

//     Ok(())
// }

use regex::Regex;

fn main() {
    let input = r#"
[playlist-started]https://www.youtube.com/watch?v=-tt2ZmH-3uc
[playlist-started]https://www.youtube.com/watch?v=Xsvg_WatcaE
[playlist-started]https://www.youtube.com/watch?v=YAMbQoArGSo
[playlist-started]https://www.youtube.com/watch?v=J9j78BNAlBg
[playlist-started]https://www.youtube.com/watch?v=B3W4_3exi5o
[playlist-started]PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M;Ongezellig
"#;

    let url_re = Regex::new(r"^\[playlist-started\](https?://[^\s]+)$").unwrap();
    let meta_re = Regex::new(r"^\[playlist-started\]([^;]+);(.+)$").unwrap();

    let mut urls = Vec::new();
    let mut playlist_id = String::new();
    let mut playlist_title = String::new();

    for line in input.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(caps) = url_re.captures(line) {
            urls.push(caps[1].to_string());
        } else if let Some(caps) = meta_re.captures(line) {
            playlist_id = caps[1].to_string();
            playlist_title = caps[2].to_string();
        }
    }

    println!("URLs:\n{:#?}", urls);
    println!("Playlist ID: {}", playlist_id);
    println!("Playlist Title: {}", playlist_title);
}


// yt-dlp --quiet --progress --flat-playlist --print "playlist:[playlist-started]%(id)s;%(title)s" --print "video:[playlist-started]%(url)s" "https://youtube.com/playlist?list=PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M&si=Lf2wNtv6hpcAH3us"
