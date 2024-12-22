#![feature(let_chains)]
mod traits;

use clap::Parser;
use clap_verbosity_flag::Verbosity;

#[derive(Debug, Parser)]
pub struct RawArgs {
    /// Version of `planet.osm.pbf` to read.
    /// Available to read from
    /// https://planet.openstreetmap.org/pbf/
    #[arg(long, default_value = "241206")]
    version: String,

    #[command(flatten)]
    pub verbose: Verbosity,
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub version: String,
}

impl From<RawArgs> for ParsedArgs {
    fn from(RawArgs { version, .. }: RawArgs) -> Self {
        Self { version }
    }
}
