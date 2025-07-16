use anyhow::Result;
use duct::cmd;
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    let messages = vec!["first", "second", "third"];

    let handles = messages.into_iter().map(|msg| {
        let msg = msg.to_string();
        task::spawn_blocking(move || {
            // Simulate yt-dlp with echo
            cmd!("sh", "-c", format!("echo downloading {msg}; sleep 1; echo done {msg}"))
                .stderr_to_stdout()
                .stdout_capture()
                .read()
        })
    });

    let results = futures::future::try_join_all(handles).await?;

    for (i, output) in results.into_iter().enumerate() {
        println!("=== Output {} ===\n{}", i + 1, output?);
    }

    Ok(())
}

// yt-dlp --quiet --progress --flat-playlist --print "playlist:[playlist-started]%(id)s;%(title)s" --print "video:[playlist-started]%(url)s" "https://youtube.com/playlist?list=PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M&si=Lf2wNtv6hpcAH3us"
