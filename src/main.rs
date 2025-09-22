mod osm;

use crate::osm::{get_unweighted_cyclable_graphmap_from_elements, read_to_nodes_coord};
use anyhow::{anyhow, Result};
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use geo::{Coord, Rect};
use itertools::Itertools;
use log::{debug, info};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::PathBuf,
};

async fn insert_node_ids(pool: &PgPool, nodes: Vec<i64>) -> Result<()> {
    info!("Inserting nodes");
    let query = r#"
        INSERT INTO osm_node(id)
        SELECT * FROM UNNEST($1::bigint[])
        ON CONFLICT DO NOTHING
    "#;

    let updated = sqlx::query(query)
        .bind(nodes)
        .execute(pool)
        .await?
        .rows_affected();

    info!("Inserted {} nodes", updated);

    Ok(())
}

async fn insert_edge_ids(pool: &PgPool, edges_unzipped: (Vec<i64>, Vec<i64>)) -> Result<()> {
    let (source_node_ids, target_node_ids) = edges_unzipped;

    info!("Inserting edges");

    let query = r#"
        INSERT INTO osm_node_edge(source_node_id,target_node_id)
        SELECT * FROM UNNEST($1::bigint[], $2::bigint[])
        ON CONFLICT DO NOTHING
    "#;

    let updated = sqlx::query(query)
        .bind(source_node_ids)
        .bind(target_node_ids)
        .execute(pool)
        .await?
        .rows_affected();

    info!("Inserted {} edges", updated);

    Ok(())
}

async fn insert_ways(
    pool: &PgPool,
    nodes: Vec<i64>,
    edges_unzipped: (Vec<i64>, Vec<i64>),
) -> Result<()> {
    insert_node_ids(pool, nodes).await?;
    insert_edge_ids(pool, edges_unzipped).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = RawArgs::try_parse()?;

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .try_init()?;

    // ThreadPoolBuilder::new().num_threads(3).build_global()?;

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
                let edges_unzipped: (Vec<_>, Vec<_>) =
                    graph.all_edges().map(|(a, b, _)| (a, b)).unzip();

                info!("Graph ready");

                insert_ways(&pool, nodes, edges_unzipped).await?;
            }
            Extract::Coordinates { map } => {
                info!("Reading all nodes from {:?}", map);

                let cycleable_node_ids = query_node_ids(&pool).await?;

                info!("Reading nodes");
                let map =
                    read_to_nodes_coord(&map, |node_id| cycleable_node_ids.contains(node_id))?;
                info!("Read nodes");

                // only keep node_ids we can cycle, which we updated in our database earlier.

                let (node_ids, xs, ys): (Vec<i64>, Vec<f64>, Vec<f64>) = map
                    .into_iter()
                    .map(|(node_id, coord)| (node_id, coord.x, coord.y))
                    .multiunzip();

                info!("Inserting {} coords", node_ids.len());

                let query = r#"
                        UPDATE osm_node AS t
                        SET coord = ST_SetSRID(ST_Point(lon, lat), 4326)
                        FROM UNNEST($1::bigint[], $2::double precision[], $3::double precision[]) AS params(id, lon, lat)
                        WHERE t.id = params.id
                    "#;

                let updated = sqlx::query(query)
                    .bind(node_ids)
                    .bind(xs)
                    .bind(ys)
                    .execute(&pool)
                    .await?
                    .rows_affected();

                info!("Inserted {} coordinates", updated);
            }
            Extract::Elevations { tiffs } => {
                // read a tiff, get bounding rect, query for containing nodes, get elevations
                for tiff in &tiffs {
                    info!("Reading elevations from {:?}", tiff);

                    let file_in = BufReader::new(File::open(tiff)?);
                    let geotiff = geotiff::GeoTiff::read(file_in)?;
                    let rect = geotiff.model_extent();

                    let rows = query_containing_coords(&pool, rect).await?;

                    if rows.len() == 0 {
                        info!("No coordinates, skipping");
                        return Ok(());
                    }

                    let find_elevation = |coord: &Coord| {
                        geotiff
                            .get_value_at::<f64>(&coord, 0)
                            .ok_or_else(|| anyhow!("Expected to find value at {:?}", coord))
                    };

                    update_elevations(&pool, rows, find_elevation).await?;
                }
            }
        },
        _ => {
            todo!()
        }
    }

    return Ok(());
}

async fn update_elevations(
    pool: &PgPool,
    rows: HashMap<i64, Coord>,
    retrieve_elevation: impl Fn(&Coord) -> Result<f64>,
) -> Result<()> {
    let size = rows.len();

    let (node_ids, elevations): (Vec<i64>, Vec<f64>) = rows
        .into_iter()
        .map(|(node_id, coord)| -> Result<(i64, f64)> {
            Ok((node_id, retrieve_elevation(&coord)?))
        })
        // try_fold might be okay here but cbf learning
        .fold(
            Ok((Vec::with_capacity(size), Vec::with_capacity(size))),
            |accu, curr| match (accu, curr) {
                (Ok(mut accu), Ok(curr)) => {
                    accu.0.push(curr.0);
                    accu.1.push(curr.1);
                    Ok(accu)
                }
                (Err(error), _) => Err(error),
                (_, Err(error)) => Err(error),
            },
        )?;

    info!("Updating elevations");
    let query = r#"
        UPDATE osm_node as t
        SET elevation = el
        FROM UNNEST($1::bigint[], $2::double precision[])
        AS params(id, el)
        WHERE t.id = params.id
    "#;

    let updated = sqlx::query(query)
        .bind(node_ids)
        .bind(elevations)
        .execute(pool)
        .await?
        .rows_affected();

    info!("Updated {} elevations", updated);

    Ok(())
}

async fn query_containing_coords(pool: &PgPool, rect: Rect) -> Result<HashMap<i64, Coord>> {
    info!("Querying containing coords");
    let query = r#"
        SELECT id, ST_X(coord) as x, ST_Y(coord) as Y FROM osm_node
        WHERE elevation IS NULL AND coord IS NOT NULL
        AND ST_Within(coord, ST_MakeEnvelope($1, $2, $3, $4, 4326))
    "#;

    let min = rect.min();
    let max = rect.max();
    let rows: HashMap<i64, Coord> = sqlx::query(query)
        .bind(min.x)
        .bind(min.y)
        .bind(max.x)
        .bind(max.y)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| -> Result<(i64, Coord)> {
            let id: i64 = row.try_get("id")?;
            let x: f64 = row.try_get("x")?;
            let y: f64 = row.try_get("y")?;
            let coord = Coord { x, y };
            Ok((id, coord))
        })
        .try_collect()?;

    let size = rows.len();
    info!("Queried {} containing coords", size);

    Ok(rows)
}

async fn query_node_ids(pool: &PgPool) -> Result<HashSet<i64>> {
    info!("Querying cyclable nodes");
    let cycleable_node_ids: HashSet<i64> =
        sqlx::query(r#"SELECT id FROM osm_node WHERE coord IS NULL"#)
            .fetch_all(pool)
            .await?
            .iter()
            .map(|row| row.try_get::<i64, &str>("id"))
            .try_collect()?;

    info!("Queried {} cyclable nodes", cycleable_node_ids.len());

    Ok(cycleable_node_ids)
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
