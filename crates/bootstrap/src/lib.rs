#![feature(iter_collect_into)]
mod elevation;
mod sql;

use anyhow::{anyhow, Result};
use elevation::{lookup_elevations, ElevationRequestBody};
use geo::{Distance, Haversine};
use itertools::Itertools;
use log::info;
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::prelude::{DiGraphMap, UnGraphMap};
use sql::get_nodes_from_db;
use sqlx::PgPool;
use std::fs::{self, File};
use std::hash::Hash;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::Arc;
use std::{collections::HashMap, fs::create_dir_all};
use tokio::sync::Mutex;
use traits::{CollectTuples, IntoJoinConcurrently, IntoNodeIdPoint, ParMapCollect};

pub type NodeId = i64;
pub type Elevation = f64;
pub type Point = geo::Point<f64>;

// These types are what we store in the files
pub type Points = HashMap<NodeId, Point>;
pub type Elevations = HashMap<NodeId, f64>;
pub type Edges = Vec<(NodeId, NodeId)>;

const OSM_INPUT_FILE: &str = "planet.osm.pbf";
const POINTS_FILE: &str = "data/points.postcard";
const EDGES_FILE: &str = "data/edges.postcard";
const ELEVATIONS_FILE: &str = "data/elevations.json";

// fn create_elements() -> Result<ElementReader<BufReader<File>>> {
//     Ok(ElementReader::from_path(OSM_INPUT_FILE)?)
// }

// fn create_graph() -> Result<UnGraphMap<i64, ()>> {
//     let value = if fs::exists(EDGES_FILE)? {
//         info!("{EDGES_FILE} exists, reading");
//         let edges = read_postcard_data::<Edges>(EDGES_FILE)?;
//         edges
//             .into_iter()
//             .map(|(left, right)| (left, right, ()))
//             .collect()
//     } else {
//         info!("{EDGES_FILE} does not exist, creating");
//         let graph = get_unweighted_cyclable_graphmap_from_elements(create_elements()?);
//         let edges = graph
//             .all_edges()
//             .map(|(left, right, _)| (left, right))
//             .collect_vec();
//         let contents = postcard::to_stdvec(&edges)?;
//         fs::write(EDGES_FILE, contents.as_slice())?;
//         graph
//     };

//     Ok(value)
// }

// fn create_points(graph: &UnGraphMap<NodeId, ()>) -> Result<Points> {
//     let value = if fs::exists(POINTS_FILE)? {
//         info!("{POINTS_FILE} exists, reading");
//         read_postcard_data::<Points>(POINTS_FILE)?
//     } else {
//         info!("{POINTS_FILE} does not exist, creating");
//         let points = create_elements()?.par_map_collect(|element| {
//             get_points_by_node_id(element, |node_id| graph.contains_node(*node_id))
//         });
//         let contents = postcard::to_stdvec(&points)?;
//         fs::write(POINTS_FILE, contents.as_slice())?;
//         points
//     };

//     Ok(value)
// }

#[derive(Debug, Default)]
pub struct EdgeWeight {
    pub distance: f64,
    pub gradient: f64,
}

pub async fn get() -> Result<(HashMap<i64, (geo::Point, f64)>, DiGraphMap<i64, EdgeWeight>)> {
    let url = "postgres://user:password@localhost:5432/elevated-cycling";
    let pool = PgPool::connect(url).await?;

    let client = reqwest::Client::new();

    // read ways nodeids and nodes with points from osm ALWAYS
    // SQL add nodes

    // Read from files, otherwise create them.
    // info!("Creating graph");
    // let graph = create_graph()?;

    // info!("Creating points");
    // let points = create_points(&graph)?;

    // info!("Creating elevations");
    // let elevations = create_elevations(&client, &points).await?;

    // info!("Combine elevations and points");
    // let nodes = points
    //     .iter()
    //     .map(|(node_id, point)| {
    //         let elevation = *elevations
    //             .get(node_id)
    //             .ok_or_else(|| anyhow!("Expected to find this here but didn't"))?;

    //         Ok((*node_id, (*point, elevation)))
    //     })
    //     .collect::<Result<HashMap<_, _>>>()?;

    // info!("Create graph with weighted edges");
    // let graph: DiGraphMap<_, _> = graph
    //     .all_edges()
    //     .flat_map(|(left_node_id, right_node_id, _)| {
    //         let left_point = points.get(&left_node_id).unwrap();
    //         let right_point = points.get(&right_node_id).unwrap();
    //         let distance_diff = Haversine::distance(*left_point, *right_point);

    //         let left_elevation = elevations.get(&left_node_id).unwrap();
    //         let right_elevation = elevations.get(&right_node_id).unwrap();
    //         let elevation_diff = left_elevation - right_elevation;

    //         let gradient = elevation_diff / distance_diff;

    //         let edge_left = EdgeWeight {
    //             distance: distance_diff,
    //             gradient,
    //         };

    //         let edge_right = EdgeWeight {
    //             distance: distance_diff,
    //             gradient: -gradient,
    //         };

    //         [
    //             (left_node_id, right_node_id, edge_left),
    //             (right_node_id, left_node_id, edge_right),
    //         ]
    //     })
    //     .collect();

    // Ok((nodes, graph))
    Ok((Default::default(), Default::default()))
}

// fn get_elevations() -> Result<HashMap<i64, f64>> {
//     let value = if fs::exists(ELEVATIONS_FILE).unwrap() {
//         info!("{ELEVATIONS_FILE} exists, reading");

//         let contents = BufReader::new(File::open(ELEVATIONS_FILE)?);

//         serde_json::Deserializer::from_reader(contents)
//             .into_iter::<Elevations>()
//             .flat_map(|elevations| elevations.unwrap().into_iter())
//             .collect::<Elevations>()
//     } else {
//         info!("{ELEVATIONS_FILE} does not exist, setting empty elevations");

//         Elevations::default()
//     };

//     Ok(value)
// }

// // We need to write incrementally to the file as it receives data from the API,
// // Because with a huge dataset in memory, the responses from the API are very very slow.
// //
// // Using JSON because it's easily to read & write stream as separated values.
// // Will consider postcard COBS flavour but not yet.
// // TODO: Handle all errors
// // TODO: this isn't reading all the elements from a file...
// async fn create_elevations(
//     client: &reqwest::Client,
//     nodes: &HashMap<i64, Point>,
// ) -> Result<Elevations> {
//     const CONCURRENCY: usize = 64;
//     const CHUNKS: usize = 100;

//     let mut elevations_existing = get_elevations()?;

//     let total_nodes = nodes.len() - elevations_existing.len();

//     info!(
//         "nodes: {}, existing: {}, remaining: {}",
//         nodes.len(),
//         elevations_existing.len(),
//         total_nodes
//     );

//     let total = total_nodes / CHUNKS;
//     info!("{total} chunks remaining");

//     // Write to one file as each future completes.
//     let writer = Arc::new(Mutex::new(File::create(ELEVATIONS_FILE).unwrap()));

//     nodes
//         .iter()
//         // Only fetch the NodeId's we're missing.
//         .filter(|(node_id, _)| !elevations_existing.contains_key(*node_id))
//         .chunks(CHUNKS)
//         .into_iter()
//         .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
//         .enumerate()
//         .map(|(index, (node_ids, points))| {
//             let writer = writer.clone();
//             let count = index + 1;

//             async move {
//                 info!("elevations: chunk {} of {}: fetching", count, total);
//                 let response =
//                     lookup_elevations(&client, ElevationRequestBody::from_iter(points)).await?;
//                 let elevations = node_ids
//                     .into_iter()
//                     .zip_eq(response)
//                     .collect::<Elevations>();
//                 let contents = serde_json::to_vec(&elevations)?;

//                 writer.lock().await.write_all(&contents.as_slice())?;
//                 info!("elevations: chunk {} of {}: complete", count, total);
//                 Result::<HashMap<NodeId, Elevation>>::Ok(elevations)
//             }
//         })
//         .join_concurrently_result::<Vec<_>, _>(CONCURRENCY)
//         .await?
//         .into_iter()
//         .flatten()
//         .collect_into(&mut elevations_existing);

//     Ok(elevations_existing)
// }

// /// Creates a `HashMap` of points where `node_id`'s
// fn get_points_by_node_id(
//     element: Element<'_>,
//     contains: impl Fn(&i64) -> bool,
// ) -> HashMap<i64, Point> {
//     element
//         .node_id_point()
//         .filter(|(node_id, _)| contains(node_id))
//         .map(|(node_id, point)| {
//             let mut hashmap = HashMap::<i64, Point>::with_capacity(1);
//             hashmap.insert(node_id, point);
//             hashmap
//         })
//         .unwrap_or_default()
// }

// /// Creates an undirected, unweighted graph from all ways in an Open Street Maps PBF.
// fn get_unweighted_cyclable_graphmap_from_elements<R>(
//     elements: ElementReader<R>,
// ) -> UnGraphMap<i64, ()>
// where
//     R: Read + Send,
// {
//     elements
//         .par_map_reduce(
//             get_cyclable_node_ids_from_element,
//             || UnGraphMap::default(),
//             |mut accu, curr| {
//                 accu.extend(curr.all_edges());
//                 accu
//             },
//         )
//         .unwrap()
// }

// /// Creates an undirected `GraphMap` when an element is a way.
// fn get_cyclable_node_ids_from_element(element: Element<'_>) -> UnGraphMap<i64, ()> {
//     match element {
//         Element::Way(way) => Some(way),
//         _ => None,
//     }
//     .filter(|way| contains_cycleable_tags(way.tags()))
//     .map(|way| {
//         way.refs()
//             .tuple_windows::<(_, _)>()
//             .map(|(from, to)| (from, to, ()))
//             .collect::<UnGraphMap<_, _>>()
//     })
//     .unwrap_or_default()
// }

// /// Returns true when a combination of any tags indicate it is cyclable.
// /// Inferred from https://wiki.openstreetmap.org/wiki/Map_features
// fn contains_cycleable_tags(tags: TagIter<'_>) -> bool {
//     let mut highway_footway = false;
//     let mut bicycle_yes = false;

//     for tag in tags {
//         match tag {
//             ("highway", "footway") => {
//                 highway_footway = true;
//             }
//             ("bicycle", "yes") => {
//                 bicycle_yes = true;
//             }
//             _ => {}
//         }

//         if highway_footway && bicycle_yes {
//             return true;
//         }

//         if cyclable_way(tag) {
//             return true;
//         }
//     }

//     false
// }

// /// Returns true when a tag for a way is cyclable.
// /// Inferred from https://wiki.openstreetmap.org/wiki/Map_features
// fn cyclable_way(pair: (&str, &str)) -> bool {
//     matches!(
//         pair,
//         (
//             "highway",
//             "trunk"
//                 | "primary"
//                 | "secondary"
//                 | "tertiary"
//                 | "residential"
//                 | "living_street"
//                 | "service"
//                 | "pedestrian"
//                 | "road"
//                 | "cycleway"
//         ) | ("cycleway", _)
//             | ("bicycle_road", "yes")
//     )
// }
