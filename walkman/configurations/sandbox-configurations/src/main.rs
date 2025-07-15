use duct::cmd;
use tokio::task;
use anyhow::Result;

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
