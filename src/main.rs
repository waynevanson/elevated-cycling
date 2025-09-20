mod osm;
mod traits;

use crate::osm::{derive_coords_from_osm_pbf, get_unweighted_cyclable_graphmap_from_elements};
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use geo::{Coord, Intersects};
use itertools::Itertools;
use log::{info, warn};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

// set up dev shell

const READ_BUF_CAPACITY: usize = 8usize.pow(8);

// maybe don't use paths and instead just push to stdout

const TABLE_NODE_ID_COORDS: TableDefinition<i64, (f64, f64)> =
    TableDefinition::new("node_id_coords");

const TABLE_NODE_ID_ELEVATION: TableDefinition<i64, f64> =
    TableDefinition::new("node_id_elevations");

const TABLE_NODE_ID_EDGES_DIRECTED: TableDefinition<i64, i64> =
    TableDefinition::new("node_id_neighbours");

#[tokio::main]
async fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    match args.subcommand {
        SubCommand::Bootstrap(extract) => {
            match extract {
                Extract::Ways { map } => {
                    let graph = get_unweighted_cyclable_graphmap_from_elements(&map)?;
                    info!("Reading {:?} to create a graph of cycleable node", map);
                }
                // todo: reiterate for a graph
                Extract::Coordinates { map } => {
                    let db = Database::create("db.redb")?;

                    info!("Writing to DB");
                    let transaction = db.begin_write()?;
                    {
                        let mut table = transaction.open_table(TABLE_NODE_ID_EDGES_DIRECTED)?;

                        for (key, value, _) in graph.all_edges() {
                            table.insert(key, value)?;
                        }
                    };
                    transaction.commit()?;
                    info!("Written to DB");

                    let cyclable_node_ids = graph.nodes().collect::<HashSet<i64>>();

                    info!("Reading {:?} to create all the coords", map);
                    let points = derive_coords_from_osm_pbf(&map, &cyclable_node_ids)?;

                    info!("Writing to DB");
                    let transaction = db.begin_write()?;
                    {
                        let mut table = transaction.open_table(TABLE_NODE_ID_COORDS)?;

                        for (node_id, Coord { x, y }) in points {
                            table.insert(node_id, (x, y))?;
                        }
                    };
                    transaction.commit()?;
                    info!("Written to DB");
                }
                Extract::Elevations { tiffs } => {
                    let db = Database::create("db.redb")?;

                    let existing_elevations: HashSet<i64> = {
                        let transaction = db.begin_read()?;
                        let table = transaction.open_table(TABLE_NODE_ID_ELEVATION)?;
                        table.iter()?.map_ok(|a| a.0.value()).try_collect()?
                    };

                    let mut coords = {
                        info!("Reading coordinates form database");
                        let transaction = db.begin_read()?;
                        let table = transaction.open_table(TABLE_NODE_ID_COORDS)?;

                        let coords = table
                            .iter()?
                            .map_ok(|value| {
                                let node_id = value.0.value();
                                let coord = Coord::from(value.1.value());
                                (node_id, coord)
                            })
                            .filter_ok(|entry| !existing_elevations.contains(&entry.0))
                            .fold_ok(
                                HashMap::with_capacity(table.len()? as usize),
                                |mut acc, (k, v)| {
                                    acc.insert(k, v);
                                    acc
                                },
                            )?;

                        coords
                    };

                    info!("Read coordinates form database");

                    for tiff in tiffs {
                        let elevations = read_geotiff_to_elevations(&mut coords, tiff)?;

                        for node_id in elevations.keys() {
                            coords.remove(node_id);
                        }

                        let transaction = db.begin_write()?;

                        {
                            let mut table = transaction.open_table(TABLE_NODE_ID_ELEVATION)?;

                            for (node_id, elevation) in elevations {
                                table.insert(node_id, elevation)?;
                            }
                        };

                        transaction.commit()?;
                    }

                    if coords.len() > 0 {
                        warn!("Still have {} coords remaining", coords.len());
                    }

                    info!("Badabing, badaboom!")
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
    coords: &HashMap<i64, Coord>,
    tiff: PathBuf,
) -> Result<HashMap<i64, f64>> {
    info!("Reading elevations from geotiff");

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

    info!("Read elevations from geotiff");

    Ok(elevations)
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
    Bootstrap(Extract),
    Circuit {},
}

// todo: both when there's no name and it's just extract
#[derive(Debug, Parser, Clone)]
pub enum Extract {
    Ways {
        #[arg(short, long)]
        map: PathBuf,
    },
    #[command(alias = "coords")]
    Coordinates {
        #[arg(short, long)]
        map: PathBuf,
    },
    Elevations {
        tiffs: Vec<PathBuf>,
    },
}
