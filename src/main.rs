#![feature(fn_traits, unboxed_closures, tuple_trait)]

mod connections;
mod elevation;
mod osm_pbf;
mod traits;

use crate::{
    elevation::{lookup_elevations, ElevationRequestBody},
    osm_pbf::{IntoCyclableNodes, IntoPointsByNodeId, NodeId},
};
use axum::{
    debug_handler,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use clap::Parser;
use elevation::ElevationsError;
use geo::Point;
use itertools::{FoldWhile, Itertools};
use osmpbf::ElementReader;
use reqwest::Client;
use serde::{Deserialize, Serialize, Serializer};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use thiserror::Error;
use traits::{CollectTuples, IntoAllSimplePaths, IntoJoinAll, PartitionResults};
use url::Url;

#[derive(Debug, Clone, Parser)]
struct Args {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct CircuitDownHillRequest {
    latitude: f64,
    longitude: f64,
    max_radius: f64,
}

#[derive(Debug, Clone, Serialize)]
struct CircuitDownHillResponse {
    #[serde(serialize_with = "serialize_points")]
    coordinates: Vec<Point>,
}

#[derive(Debug, Clone, Serialize)]
struct MapBBCode(#[serde(serialize_with = "serialize_points")] Vec<Point>);

fn serialize_points<S>(points: &Vec<Point>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    points
        .into_iter()
        .map(|point| {
            [
                point.x().to_string(),
                ",".to_string(),
                point.y().to_string(),
            ]
            .into_iter()
            .collect::<String>()
        })
        .intersperse(" ".to_string())
        .collect::<String>()
        .serialize(serializer)
}

type Elevation = f64;

fn get_reward_factor<'a>(
    path: &Vec<NodeId>,
    get_elevation_by_id: impl Fn(&NodeId) -> Option<&'a Elevation>,
) -> f32 {
    let size = path.len() as f32;
    let max_position = path
        .iter()
        .map(get_elevation_by_id)
        .enumerate()
        .reduce(
            |(accu_index, accu_elevation), (curr_index, curr_elevation)| {
                if accu_elevation >= curr_elevation {
                    (accu_index, accu_elevation)
                } else {
                    (curr_index, curr_elevation)
                }
            },
        )
        .unwrap()
        .0 as f32;

    (size - max_position) / (size + 1.0)
}

fn get_node_id_origin<'a, I>(mut iter: I) -> Option<&'a i64>
where
    I: Iterator<Item = (&'a i64, &'a f64)>,
{
    iter.fold_while(
        None,
        |closest: Option<(&NodeId, &f64)>, (node_id, distance)| {
            if distance == &0.0 {
                FoldWhile::Done(Some((node_id, distance)))
            } else if let Some((node_id_closest, distance_closest)) = closest {
                let closest = if distance < distance_closest {
                    (node_id, distance)
                } else {
                    (node_id_closest, distance_closest)
                };

                FoldWhile::Continue(Some(closest))
            } else {
                FoldWhile::Continue(Some((node_id, distance)))
            }
        },
    )
    .into_inner()
    .map(|a| a.0)
}

#[derive(Debug, Error)]
enum RequestError {
    #[error("{0}")]
    OsmPbfError(#[from] osmpbf::Error),

    #[error("Expected to find the not origin")]
    NodeOriginNotFound,

    #[error("Expcted the paths to contain at least 1 path")]
    PathsNotFound,

    #[error("{0:?}")]
    ElevationsError(Vec<ElevationsError>),
}

impl IntoResponse for RequestError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    client: reqwest::Client,
    osm_filepath: PathBuf,
}

#[debug_handler]
async fn handler(
    State(context): State<RequestContext>,
    Json(request): Json<CircuitDownHillRequest>,
) -> Result<String, RequestError> {
    let create_elements = || ElementReader::from_path(&context.osm_filepath).unwrap();

    let origin = Point::from((request.latitude, request.longitude));

    let points_by_node_id =
        create_elements().into_points_by_node_id_within_range(&origin, request.max_radius)?;

    let nodes_with_distance = points_by_node_id
        .iter()
        .map(|(node_id, (_, distance))| (node_id, distance));

    let node_id_origin =
        *get_node_id_origin(nodes_with_distance).ok_or(RequestError::NodeOriginNotFound)?;

    let graph_node_ids = create_elements().into_cyclable_nodes(&points_by_node_id)?;

    let elevation_by_node_id: HashMap<NodeId, Elevation> = graph_node_ids
        .nodes()
        .filter_map(|node_id| {
            points_by_node_id
                .get(&node_id)
                .map(|point_distance| (node_id, point_distance.0))
        })
        .chunks(1_000)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<NodeId>, Vec<_>>())
        .map(|(node_ids, points)| async {
            lookup_elevations(&context.client, ElevationRequestBody::from_iter(points))
                .await
                .map(|elevations| {
                    node_ids
                        .into_iter()
                        .zip_eq(elevations)
                        .collect::<HashMap<NodeId, Elevation>>()
                })
        })
        .join_all()
        .await
        .into_iter()
        .partition_results::<Vec<_>, Vec<_>>()
        .map_err(RequestError::ElevationsError)?
        .into_iter()
        .flatten()
        .collect();

    // let forks_only = connections(&graph_node_ids);

    let paths = graph_node_ids
        .into_all_simple_paths::<Vec<_>>(node_id_origin, node_id_origin, 0, None)
        .map(|mut path| {
            path.push(node_id_origin);
            path
        });

    let path = paths
        .map(|path| {
            let factor = get_reward_factor(&path, |node_id| elevation_by_node_id.get(node_id));
            (path, factor)
        })
        .reduce(|(accu_path, accu_factor), (curr_path, curr_factor)| {
            if accu_factor >= curr_factor {
                (accu_path, accu_factor)
            } else {
                (curr_path, curr_factor)
            }
        })
        .ok_or(RequestError::PathsNotFound)?
        .0;

    let points = path
        .iter()
        .map(|node_id| points_by_node_id.get(node_id).unwrap())
        .map(|(point, _)| point)
        .cloned()
        .collect_vec();

    let stringified: String = points
        .into_iter()
        .map(|point| {
            [
                point.x().to_string(),
                ",".to_string(),
                point.y().to_string(),
            ]
            .into_iter()
            .collect::<String>()
        })
        .intersperse(" ".to_string())
        .collect();

    let stringified: String = ["[map]".to_string(), stringified, "[/map]".to_string()]
        .into_iter()
        .collect::<String>();

    let mut url = Url::from_str("https://d10k44lwpk7bmb.cloudfront.net/index.html").unwrap();
    url.query_pairs_mut().append_pair("mapbbcode", &stringified);

    Ok(url.to_string())
}

#[tokio::main]
async fn main() -> Result<(), RequestError> {
    let file_path = Args::parse().file_path;
    let client = Client::new();

    let context = RequestContext {
        client,
        osm_filepath: file_path,
    };

    // build our application with a route
    let app = Router::new().route("/", get(handler).with_state(context));

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
