mod osm;
mod traits;

use crate::osm::{derive_coords_from_osm_pbf, get_unweighted_cyclable_graphmap_from_elements};
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use geo::{Coord, Intersects};
use log::info;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter, Read},
    path::{Path, PathBuf},
};

const READ_BUF_CAPACITY: usize = 8usize.pow(8);

// maybe don't use paths and instead just push to stdout
const DEFAULT_PATH_MAP_OSM_PBF: &str = "map.osm.pbf";
const DEFAULT_PATH_COORDS: &str = ".coords.postcard";
const DEFAULT_PATH_ELEVATIONS: &str = ".elevations.postcard";

#[tokio::main]
async fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    match args.subcommand {
        SubCommand::Extract(extract) => {
            match extract {
                // todo: reiterate for a graph
                Extract::Coordinates { map, coords } => {
                    let graph = get_unweighted_cyclable_graphmap_from_elements(&map)?;

                    let cyclable_node_ids = graph.nodes().collect::<HashSet<i64>>();
                    let points = derive_coords_from_osm_pbf(&map, &cyclable_node_ids)?;

                    info!(
                        "Serializing {} units of data from memory to {:?}",
                        points.len(),
                        coords
                    );
                    // ~ 5 seconds

                    let out_file = BufWriter::new(File::create(&coords)?);

                    postcard::to_io(&points, out_file)?;

                    info!("Serialized to {:?}", coords)
                }
                Extract::Elevations {
                    coords,
                    tiffs,
                    elevations,
                } => {
                    let mut coords = read_coords(coords)?;
                    let mut all_elevations = HashMap::<i64, f64>::with_capacity(coords.len());

                    for tiff in tiffs {
                        let elevations = read_geotiff_to_elevations(&mut coords, tiff)?;

                        for node_id in elevations.keys() {
                            coords.remove(node_id);
                        }

                        all_elevations.extend(elevations);
                    }

                    let out_file = BufWriter::new(File::create(&elevations)?);

                    postcard::to_io(&coords, out_file)?;

                    info!("Serialized to {:?}", coords)
                }
            }
        }
        SubCommand::Circuit { .. } => {
            // retrieve the node graph, node to coords map and elevation.
            // create directed graph with edges being elevation
            // the algorithm.
            todo!()
        }
    }

    return Ok(());
}

fn read_geotiff_to_elevations(
    coords: &mut HashMap<i64, Coord>,
    tiff: PathBuf,
) -> Result<HashMap<i64, f64>> {
    let file_in = BufReader::with_capacity(READ_BUF_CAPACITY, File::open(tiff)?);

    let geotiff = geotiff::GeoTiff::read(file_in)?;

    let rect = geotiff.model_extent();

    let elevations = coords
        .iter()
        .filter(|(_, coord)| rect.intersects(*coord))
        .filter_map(|(node_id, coord)| {
            geotiff
                .get_value_at::<f64>(&coord, 1)
                .map(|elevation| (*node_id, elevation))
        })
        .collect::<HashMap<i64, f64>>();

    Ok(elevations)
}

fn read_coords(path: impl AsRef<Path>) -> Result<HashMap<i64, Coord>> {
    info!("Reading and deserializing data from {:?}", path.as_ref());
    // ~21 seconds

    let mut buf = Vec::new();
    let mut cache_file = BufReader::with_capacity(READ_BUF_CAPACITY, File::open(&path)?);
    cache_file.read_to_end(&mut buf)?;

    let coords: HashMap<i64, Coord> = postcard::from_bytes(&buf)?;

    info!(
        "Deserialized a total of {} units into memory from {:?} ",
        coords.len(),
        path.as_ref()
    );

    Ok(coords)
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
        #[arg(short, long, default_value = DEFAULT_PATH_COORDS)]
        cache: PathBuf,
    },
}

// todo: both when there's no name and it's just extract
#[derive(Debug, Parser, Clone)]
pub enum Extract {
    #[command(alias = "coords")]
    Coordinates {
        #[arg(short, long, default_value = DEFAULT_PATH_MAP_OSM_PBF)]
        map: PathBuf,

        #[arg(short, long, default_value = DEFAULT_PATH_COORDS)]
        coords: PathBuf,
    },
    Elevations {
        #[arg(short, long, default_value = DEFAULT_PATH_COORDS)]
        coords: PathBuf,

        #[arg(short, long, default_value = DEFAULT_PATH_ELEVATIONS)]
        elevations: PathBuf,

        tiffs: Vec<PathBuf>,
    },
}
