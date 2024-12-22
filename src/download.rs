use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use futures::StreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue, RANGE},
    IntoUrl,
};
use std::path::{Path, PathBuf};
use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
};

pub async fn download_with_cachable(
    client: &reqwest::Client,
    url: impl IntoUrl,
    file_path: impl AsRef<Path>,
) -> Result<()> {
    let file_path = PathBuf::from(".cached").join(&file_path);

    // Check existing file size
    let existing_size = if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
        metadata.len()
    } else {
        0
    };

    if existing_size > 0 {
        println!(
            "Reading from existing file of size {}",
            ByteSize::b(existing_size)
        );
    }

    // Add `Range` header for resuming download
    let mut headers = HeaderMap::new();
    if existing_size > 0 {
        headers.insert(
            RANGE,
            HeaderValue::from_str(&format!("bytes={}-", existing_size))?,
        );
    }

    let response = client.get(url.as_str()).send().await?;

    // Ensure partial content or complete download
    if !(response.status().is_success() || response.status().as_u16() == 206) {
        return Err(anyhow!("Failed to download: HTTP {}", response.status()));
    }

    let total_size = response.content_length();

    let suffix = total_size
        .map(ByteSize::b)
        .map(|size| format!(" of total size {size}"))
        .unwrap_or_default();

    println!("Total size of {suffix}");

    println!("Downloading from {}", url.as_str());

    if let Some(parent) = PathBuf::from(&file_path).parent() {
        create_dir_all(parent).await?;
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .await?;

    file.seek(std::io::SeekFrom::Start(existing_size)).await?;

    let mut stream = response.bytes_stream();

    let mut bytes_so_far = existing_size as f64;

    let mut prev_percentage = 0f64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        if let Some(total_size) = total_size {
            bytes_so_far += chunk.len() as f64;
            let percentage = truncate_float((bytes_so_far / total_size as f64) * 100.0, 2);

            if prev_percentage != percentage {
                eprintln!("{percentage}%");
                prev_percentage = percentage;
            }
        }

        file.write_all(&chunk).await?;
    }

    Ok(())
}

fn truncate_float(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (value * factor).trunc() / factor
}
