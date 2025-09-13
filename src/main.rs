#![feature(let_chains)]
mod traits;

use crate::traits::{IntoNodeIdPoint, ParMapCollect};
use anyhow::{Error, Result};
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use geo::{Coord, Intersects, Point};
use itertools::Itertools;
use log::{info, warn};
use osmpbf::reader::ElementReader;
use std::{
    collections::HashMap,
    fs::File,
    hash::Hash,
    io::{BufReader, BufWriter, Read},
    path::{Path, PathBuf},
};

const READ_BUF_CAPACITY: usize = 8usize.pow(8);

#[tokio::main]
async fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    match args.subcommand {
        // todo: subcommand osm.pbf -> points and .tiff -> elevations
        SubCommand::Extract(extract) => {
            match extract {
                // todo: needs to be a graph
                Extract::Coordinates { map, cache } => {
                    let points = derive_coords_from_osm_pbf(map)?;

                    info!(
                        "Serializing {} units of data from memory to {:?}",
                        points.len(),
                        cache
                    );
                    // ~ 5 seconds

                    let out_file = BufWriter::new(File::create(&cache)?);

                    postcard::to_io(&points, out_file)?;

                    info!("Serialized to {:?}", cache)
                }
                Extract::Elevations { cache, tiffs } => {
                    // remove elements from this?
                    let mut points = read_points(cache)?;

                    for tiff in tiffs {
                        let file_in =
                            BufReader::with_capacity(READ_BUF_CAPACITY, File::open(tiff)?);

                        let geotiff = geotiff::GeoTiff::read(file_in)?;

                        let rect = geotiff.model_extent();

                        let elevations = points
                            .iter()
                            .map(|(node_id, point)| (node_id, point.0))
                            .filter(|(_, point)| rect.intersects(point))
                            .filter_map(|(node_id, coord)| {
                                geotiff
                                    .get_value_at::<f64>(&coord, 1)
                                    .map(|elevation| (*node_id, elevation))
                            })
                            .collect::<HashMap<i64, f64>>();

                        for node_id in elevations.keys() {
                            points.remove(node_id);
                        }
                    }

                    todo!()
                }
            }
        }
        SubCommand::Circuit { .. } => {
            todo!()
        }
    }

    return Ok(());
}

fn derive_coords_from_osm_pbf(map: PathBuf) -> Result<HashMap<i64, Point>> {
    let osm = BufReader::with_capacity(READ_BUF_CAPACITY, File::open(&map)?);
    let pbf = ElementReader::new(osm);

    info!("Extracting data from {:?} into memory", map);
    // ~31 seconds

    let points = pbf.par_map_collect(|element| {
        let mut map = HashMap::with_capacity(1);
        map.extend(element.node_id_point());
        map
    });

    info!("Extracted data from {:?} into memory", map);

    Ok(points)
}

fn read_points(path: impl AsRef<Path>) -> Result<HashMap<i64, Point>> {
    info!("Reading and deserializing data from {:?}", path.as_ref());
    // ~21 seconds

    let mut buf = Vec::new();
    let mut cache_file = BufReader::with_capacity(READ_BUF_CAPACITY, File::open(&path)?);
    cache_file.read_to_end(&mut buf)?;

    let points: HashMap<i64, Point> = postcard::from_bytes(&buf)?;

    info!(
        "Deserialized a total of {} units into memory from {:?} ",
        points.len(),
        path.as_ref()
    );

    Ok(points)
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
    #[command(subcommand)]
    /// Extracts only the data required from the `*.pbf` into a `.*.postcard` file
    Extract(Extract),
    Circuit {
        #[arg(short, long, default_value = ".cache.postcard")]
        cache: PathBuf,
    },
}

// todo: both when there's no name and it's just extract
#[derive(Debug, Parser, Clone)]
pub enum Extract {
    #[command(alias = "coords")]
    Coordinates {
        #[arg(short, long, default_value = "map.osm.pbf")]
        map: PathBuf,

        #[arg(short, long, default_value = ".cache.postcard")]
        cache: PathBuf,
    },
    Elevations {
        #[arg(short, long, default_value = ".cache.postcard")]
        cache: PathBuf,

        tiffs: Vec<PathBuf>,
    },
}
