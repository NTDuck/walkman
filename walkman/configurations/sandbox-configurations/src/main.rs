use std::{collections::HashSet, io::{Read, Write}};

use anyhow::Error;
use flate2::{read::{ZlibDecoder}, write::ZlibEncoder, Compression};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open("data.bin")
        .await?;

    let file = tokio::sync::Mutex::new(file);

    // Write section
    {
        let mut hashset = HashSet::new();
        hashset.insert("foo");
        hashset.insert("bar");
        hashset.insert("baz");

        // Serialization
        let buffer = bincode::encode_to_vec(hashset, bincode::config::standard())?;

        // Compression
        let mut compressor = ZlibEncoder::new(Vec::new(), Compression::default());
        compressor.write_all(&buffer)?;
        let buffer = compressor.finish()?;

        let mut f = file.lock().await;
        f.seek(std::io::SeekFrom::Start(0)).await?;
        f.set_len(0).await?;
        f.write_all(&buffer).await?;
        f.flush().await?;
    }

    // Read section
    {
        let mut f = file.lock().await;
        f.seek(std::io::SeekFrom::Start(0)).await?;
        
        let mut buffer = vec![];
        f.read_to_end(&mut buffer).await?;

        // Decompression
        let mut decompressor = ZlibDecoder::new(&buffer[..]);
        let mut buffer = vec![];
        decompressor.read_to_end(&mut buffer)?;

        // Deserialization
        let (hashset, _) = bincode::decode_from_slice::<HashSet<String>, _>(&buffer, bincode::config::standard())?;

        println!("Read: {:?}", hashset);
    }

    tokio::fs::remove_file("data.bin").await?;

    Ok(())
}
