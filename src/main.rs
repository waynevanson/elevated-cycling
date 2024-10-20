#![feature(fn_traits, unboxed_closures, tuple_trait)]

mod all_simple_paths;
mod connections;
mod elevation;
mod osm_pbf;
mod traits;

use crate::{
    elevation::{lookup_elevations, ElevationRequestBody},
    osm_pbf::{IntoCyclableNodes, NodeId},
};
use all_simple_paths::IntoAllSimplePaths;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use clap::Parser;
use elevation::ElevationsError;
use futures::lock::Mutex;
use itertools::{FoldWhile, Itertools};
use ordered_float::OrderedFloat;
use osm_pbf::IntoPointsByNodeId;
use osmpbf::ElementReader;
use petgraph::{prelude::UnGraphMap, visit::IntoNeighbors};
use rayon::iter::{ParallelBridge, ParallelIterator};
use reqwest::Client;
use serde::{Deserialize, Serialize, Serializer};
use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
use thiserror::Error;
use traits::{CollectTuples, IntoJoinAll, PartitionResults};
use url::Url;

#[derive(Debug, Clone, Parser)]
struct Args {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct CircuitDownHillRequest {
    latitude: OrderedFloat<f64>,
    longitude: OrderedFloat<f64>,
    max_radius: OrderedFloat<f64>,
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
pub type Point = geo::Point<OrderedFloat<f64>>;

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

fn get_node_id_origin<'a, I>(mut iter: I) -> Option<&'a NodeId>
where
    I: Iterator<Item = (&'a NodeId, &'a OrderedFloat<f64>)>,
{
    iter.fold_while(
        None,
        |closest: Option<(&'a NodeId, &'a OrderedFloat<f64>)>, (node_id, distance)| {
            if distance == &0.0 {
                FoldWhile::Done(Some((node_id, distance)))
            } else if let Some((node_id_closest, distance_closest)) = closest {
                let closest = if distance < distance_closest.into() {
                    (node_id, distance)
                } else {
                    (node_id_closest, distance_closest.into())
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

    #[error("Expected to find the origin")]
    NodeOriginNotFound,

    #[error("Expcted the paths to contain at least 1 path")]
    PathsNotFound,

    #[error("{0:?}")]
    ElevationsError(Vec<ElevationsError>),

    #[error("{0}")]
    IOError(std::io::Error),
}

impl IntoResponse for RequestError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

type Distance = OrderedFloat<f64>;

#[derive(Debug, Clone)]
pub struct RequestContext {
    reqwest: reqwest::Client,
    file_path: PathBuf,
}

// So to init the points
#[derive(Debug, Clone, Default)]
pub struct ServerCache {
    // Checks if we have this origin in our graph
    // this is the hash to see if we need to read OSM because it's not in our map.
    distances: HashMap<Point, (Distance, NodeId)>,
    points_by_node_id: HashMap<NodeId, Point>,
    map: UnGraphMap<NodeId, Distance>,
    // Checks to see if we need to call the API to get the elevation for the node_id
    // todo: cache the elevation API.
    // elevations: HashMap<NodeId, Elevation>,
}

impl ServerCache {
    fn get_node_id_origin(&self, origin: &Point, range: &Distance) -> Option<&NodeId> {
        self.distances
            .get(origin)
            .filter(|(max, _)| range <= max)
            .map(|(_, node_id)| node_id)
    }
}

#[tokio::main]
async fn main() -> Result<(), RequestError> {
    let file_path = Args::parse().file_path;
    let reqwest = Client::new();

    let context = RequestContext { reqwest, file_path };
    let cache = Arc::new(Mutex::new(ServerCache::default()));

    // build our application with a route
    let app: Router<(RequestContext, Arc<Mutex<ServerCache>>)> =
        Router::new().route("/", get(handler));

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .map_err(RequestError::IOError)?;

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app.with_state((context, cache)))
        .await
        .map_err(RequestError::IOError)?;

    Ok(())
}

// really should just keep it all lazy huh...
async fn handler<'a>(
    State((context, mutex)): State<(RequestContext, Arc<Mutex<ServerCache>>)>,
    Json(request): Json<CircuitDownHillRequest>,
) -> Result<String, RequestError> {
    let file_path = &context.file_path;
    let create_elements = || ElementReader::from_path(file_path);

    let origin = Point::from((request.latitude, request.longitude));

    let cache = &mut mutex.lock().await;

    let node_id_origin = cache
        .get_node_id_origin(&origin, &request.max_radius)
        .copied();

    // cache miss, add all the things to the cache
    let node_id_origin = node_id_origin
        .inspect(|_| println!("CACHE HITT"))
        .map(Ok::<_, RequestError>)
        .unwrap_or_else(|| {
            println!("CACHE MISS");
            let mut points_by_node_id = create_elements()?
                .into_points_by_node_id_within_range(&origin, &request.max_radius)?;

            let node_ids_distances = points_by_node_id
                .iter()
                .map(|(node_id, (_, distance))| (node_id, distance));

            let node_origin_id =
                *get_node_id_origin(node_ids_distances).ok_or(RequestError::NodeOriginNotFound)?;

            let map = create_elements()?.into_cyclable_nodes(&points_by_node_id)?;

            // remove unused node_ids
            points_by_node_id.retain(|key, _| map.contains_node(*key));

            // add everything to cache
            cache.points_by_node_id.extend(
                points_by_node_id
                    .iter()
                    .map(|(node_id, (point, _))| (node_id, point)),
            );
            cache.map.extend(map.all_edges());

            cache
                .distances
                .insert(origin, (request.max_radius, node_origin_id));

            Ok(node_origin_id)
        })?;

    // So we want the coordinates of each node in WAY
    // we need to find all the nodes in range first,
    // filter for cyclable nodes only

    let elevation_by_node_id: HashMap<NodeId, Elevation> = cache
        .map
        .nodes()
        .filter_map(|node_id| {
            cache
                .points_by_node_id
                .get(&node_id)
                .map(|pd| (node_id, pd))
        })
        .chunks(1_000)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .map(|(node_ids, points)| async {
            // todo - elevation requests expects aour special float in response
            lookup_elevations(&context.reqwest, ElevationRequestBody::from_iter(points))
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

    let paths = cache
        .map
        .into_all_simple_paths::<Vec<_>>(node_id_origin, node_id_origin, 0, None)
        .par_bridge()
        .map(|mut path| {
            path.push(node_id_origin);
            path
        });

    println!("ALL SIMPLE BOTHS DONE");

    let path = paths
        .map(|path| {
            let factor = get_reward_factor(&path, |node_id| elevation_by_node_id.get(node_id));
            (path, factor)
        })
        .reduce_with(|(accu_path, accu_factor), (curr_path, curr_factor)| {
            if accu_factor >= curr_factor {
                (accu_path, accu_factor)
            } else {
                (curr_path, curr_factor)
            }
        })
        .ok_or(RequestError::PathsNotFound)?
        .0;

    println!("BEST PATH DONE");

    let points = path
        .iter()
        .map(|node_id| cache.points_by_node_id.get(node_id).unwrap())
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
