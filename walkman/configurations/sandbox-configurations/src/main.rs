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

use async_stream::stream;
use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use std::pin::Pin;
use tokio::sync::mpsc;

fn split_streams(
    items: Vec<String>,
) -> (
    Pin<Box<dyn Stream<Item = String> + Send>>,
    Pin<Box<dyn Stream<Item = String> + Send>>,
) {
    let (tx1, mut rx1) = mpsc::channel(10);
    let (tx2, mut rx2) = mpsc::channel(10);

    tokio::spawn(async move {
        for item in items {
            if item.starts_with("#1") {
                let _ = tx1.send(item).await;
            } else if item.starts_with("#2") {
                let _ = tx2.send(item).await;
            }
        }
    });

    let stream1 = Box::pin(stream! {
        while let Some(val) = rx1.recv().await {
            yield val;
        }
    });

    let stream2 = Box::pin(stream! {
        while let Some(val) = rx2.recv().await {
            yield val;
        }
    });

    (stream1, stream2)
}

#[tokio::main]
async fn main() {
    let input = vec![
        "#1 hello".into(),
        "#2 world".into(),
        "ignored".into(),
        "#1 again".into(),
        "#2 stream".into(),
    ];

    let (mut s1, mut s2) = split_streams(input);

    println!("Stream 1:");
    while let Some(val) = s1.next().await {
        println!("{}", val);
    }

    println!("\nStream 2:");
    while let Some(val) = s2.next().await {
        println!("{}", val);
    }
}



// yt-dlp --quiet --progress --flat-playlist --print "playlist:[playlist-started]%(id)s;%(title)s" --print "video:[playlist-started]%(url)s" "https://youtube.com/playlist?list=PLYXU4Ir4-8GPeP4lKT9aevhyhbSoHR04M&si=Lf2wNtv6hpcAH3us"
