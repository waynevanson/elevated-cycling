#![feature(let_chains)]

mod traits;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct RawArgs {
    /// File path to `planet.osm.pbf`, to read map information from.
    ///
    /// Defaults to `"$PWD/planet.osm.pbf"`
    #[arg(long)]
    planet: Option<PathBuf>,

    /// Forces reading from `planet.osm.pbf`, even when a cache exists.
    #[arg(short, long)]
    force: bool,

    #[command(flatten)]
    pub verbose: Verbosity,
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub planet: PathBuf,
    pub force: bool,
}

impl From<RawArgs> for ParsedArgs {
    fn from(RawArgs { force, planet, .. }: RawArgs) -> Self {
        Self {
            force,
            planet: planet.unwrap_or_else(|| PathBuf::from("./planet.osm.pbf")),
        }
    }
}
