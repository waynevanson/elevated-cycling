mod osm;

use crate::osm::get_unweighted_cyclable_graphmap_from_elements;
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use geo::{Coord, Intersects};
use itertools::Itertools;
use log::{debug, info};
use rayon::ThreadPoolBuilder;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};
use tokio::runtime::Builder;

async fn insert_ways(
    pool: &PgPool,
    nodes: Vec<i64>,
    source_node_ids: Vec<i64>,
    target_node_ids: Vec<i64>,
) -> Result<()> {
    info!("Inserting nodes");
    let updated = sqlx::query(
        r#"INSERT INTO osm_node(id) SELECT * FROM UNNEST($1::bigint[]) ON CONFLICT DO NOTHING"#,
    )
    .bind(nodes)
    .execute(pool)
    .await?
    .rows_affected();

    info!("Inserted {} nodes", updated);

    info!("Inserting edges");
    let updated = sqlx::query(
    r#"INSERT INTO osm_node_edge(source_node_id,target_node_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[]) ON CONFLICT DO NOTHING"#
    )
    .bind(source_node_ids)
    .bind(target_node_ids)
    .execute(pool)
    .await?
    .rows_affected();

    info!("Inserted {} edges", updated);

    Ok(())
}

// set up dev shell

fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    ThreadPoolBuilder::new().num_threads(3).build_global()?;

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(3)
        .build()?;

    runtime.block_on(async {
        debug!("Connecting pool");

        let pool = PgPoolOptions::new()
            .connect("postgres://postgres:example@127.0.0.1:5432")
            .await?;

        debug!("Pool connected");

        match args.subcommand {
            SubCommand::Bootstrap(extract) => match extract {
                Extract::Ways { map } => {
                    info!("Building graph");
                    let graph = get_unweighted_cyclable_graphmap_from_elements(&map)?;
                    let nodes = graph.nodes().collect_vec();
                    let (source_node_ids, target_node_ids): (Vec<_>, Vec<_>) =
                        graph.all_edges().map(|(a, b, _)| (a, b)).unzip();

                    info!("Graph ready");

                    insert_ways(&pool, nodes, source_node_ids, target_node_ids).await?;
                }
                _ => {
                    todo!()
                }
            },
            _ => {
                todo!()
            }
        }

        return Ok(());
    })
}

fn read_geotiff_to_elevations(
    coords: &HashMap<i64, Coord>,
    tiff: PathBuf,
) -> Result<HashMap<i64, f64>> {
    info!("Reading elevations from geotiff");

    let file_in = BufReader::new(File::open(tiff)?);

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
