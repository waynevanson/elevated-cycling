#![feature(let_chains)]
mod download;
mod traits;

use crate::traits::{IntoNodeIdPoint, ParMapCollect};
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use log::info;
use osmpbf::reader::ElementReader;
use std::{collections::HashMap, fs::File, path::PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    match args.subcommand {
        SubCommand::Extract { map, cache } => {
            let osm = File::open(&map)?;
            let pbf = ElementReader::new(osm);

            info!("Extracting data from {:?} into memory", map);

            let points = pbf.par_map_collect(|element| {
                let mut map = HashMap::with_capacity(1);
                map.extend(element.node_id_point());
                map
            });

            info!(
                "Serializing {} units of data from memory to {:?}",
                points.len(),
                cache
            );

            let out_dir = File::create(&cache)?;

            postcard::to_io(&points, out_dir)?;

            info!("Serialized to {:?}", cache)
        }
    }

    return Ok(());
}

#[derive(Debug, Parser, Clone)]
pub struct RawArgs {
    #[command(flatten)]
    pub verbose: Verbosity,

    #[command(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum SubCommand {
    /// Extracts only the data required from the `*.pbf` into a `.*.postcard` file
    Extract {
        #[arg(short, long, default_value = "./map.osm.pbf")]
        map: PathBuf,

        #[arg(short, long, default_value = ".cache.postcard")]
        cache: PathBuf,
    },
}
