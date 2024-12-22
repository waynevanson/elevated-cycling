// tokio causing this error
#![allow(clippy::needless_return)]
mod download;

use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::LevelFilter;
use download::download_with_cachable;
use elevated_cycling::{ParsedArgs, RawArgs};

#[tokio::main]
async fn main() -> Result<()> {
    let args = try_get_args()?;
    let client = reqwest::Client::new();

    download_planet_osm_pbf(&client, args.version).await?;
    download_elevations(&client).await?;
    // unrar elevations

    return Ok(());
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

async fn download_planet_osm_pbf(client: &reqwest::Client, version: String) -> Result<()> {
    let file_name = format!("planet-{version}.osm.pbf");
    let url = format!("https://planet.openstreetmap.org/pbf/{file_name}");

    download_with_cachable(client, url, file_name).await?;

    Ok(())
}

async fn download_elevations(client: &reqwest::Client) -> Result<()> {
    let file_names = [
        "SRTM_NE_250m_TIF.rar",
        "SRTM_SE_250m_TIF.rar",
        "SRTM_W_250m_TIF.rar",
    ];

    for file_name in file_names {
        let url = "https://srtm.csi.cgiar.org/wp-content/uploads/files/250m/SRTM_NE_250m_TIF.rar";
        download_with_cachable(client, url, file_name).await?;
    }

    Ok(())
}
