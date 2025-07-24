use once_cell::sync::Lazy;
use regex::Regex;

static PLAYLIST_VIDEOS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(            r"\[playlist-started:url\](?P<url>[^;]+)"
).unwrap()
});

static PLAYLIST_METADATA_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[playlist-started:metadata\](?P<id>[^;]+);(?P<title>[^;]+);(?P<url>[^;]+)").unwrap()
});

fn main() {
    let lines = [
        "[playlist-started:url]https://www.youtube.com/watch?v=-tt2ZmH-3uc",
        "[playlist-started:url]https://www.youtube.com/watch?v=Xsvg_WatcaE",
        "[playlist-started:metadata]PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M;Ongezellig;https://www.youtube.com/playlist?list=PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M&si=Lf2wNtv6hpcAH3us",
    ];

    for line in &lines {
        if let Some(caps) = PLAYLIST_VIDEOS_REGEX.captures(line) {
            println!("Matched video URL: {}", &caps["url"]);
        } else if let Some(caps) = PLAYLIST_METADATA_REGEX.captures(line) {
            println!("Matched metadata:");
            println!("  ID:    {}", &caps["id"]);
            println!("  Title: {}", &caps["title"]);
            println!("  URL:   {}", &caps["url"]);
        } else {
            println!("No match for line: {line}");
        }
    }
}
