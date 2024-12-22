use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::LevelFilter;
use elevated_cycling::{ParsedArgs, RawArgs};
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use tokio::{
    fs::OpenOptions,
    io::{AsyncSeekExt, AsyncWriteExt},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = try_get_args()?;

    Ok(())
}

fn setup_logger(level: LevelFilter) -> Result<()> {
    env_logger::Builder::new().filter_level(level).try_init()?;
    Ok(())
}

fn try_get_args() -> Result<ParsedArgs> {
    let raw_args = RawArgs::try_parse()?;

    setup_logger(raw_args.verbose.log_level_filter())?;

    let args = ParsedArgs::from(raw_args);

    Ok(args)
}

async fn download_planet_osm_pbf() -> Result<()> {
    let client = reqwest::Client::new();
    let version = "241206";

    let file_name = format!("planet-{version}.osm.pbf");
    let file_path = format!(".cached/{file_name}");
    let url = format!("https://planet.openstreetmap.org/pbf/{file_name}",);

    // Check existing file size
    let existing_size = if let Ok(metadata) = tokio::fs::metadata(file_path).await {
        metadata.len()
    } else {
        0
    };

    // Add `Range` header for resuming download
    let mut headers = HeaderMap::new();
    if existing_size > 0 {
        headers.insert(
            RANGE,
            HeaderValue::from_str(&format!("bytes={}-", existing_size))?,
        );
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&url)
        .await?;

    file.seek(std::io::SeekFrom::Start(existing_size)).await?;

    let mut stream = client.get(&url).send().await?.bytes_stream();

    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }

    Ok(())
}
