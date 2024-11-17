use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{LevelFilter, Verbosity};
use std::path::PathBuf;

fn main() -> Result<()> {
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

#[derive(Debug, Parser)]
struct RawArgs {
    /// File path to `planet.osm.pbf`, to read map information from.
    ///
    /// Defaults to `"$PWD/planet.osm.pbf"`
    #[arg(long)]
    planet: Option<PathBuf>,

    /// Forces reading from `planet.osm.pbf`, even when a cache exists.
    #[arg(short, long)]
    force: bool,

    #[command(flatten)]
    verbose: Verbosity,
}

#[derive(Debug)]
struct ParsedArgs {
    planet: PathBuf,
    force: bool,
}

impl From<RawArgs> for ParsedArgs {
    fn from(RawArgs { force, planet, .. }: RawArgs) -> Self {
        Self {
            force,
            planet: planet.unwrap_or_else(|| PathBuf::from("./planet.osm.pbf")),
        }
    }
}
