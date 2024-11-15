use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Args {
    #[arg()]
    planet_osm_pbf: Option<PathBuf>,

    force_planet: bool,
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    // read from dotenv
    println!("Hello, world!");

    Ok(())
}
