#![feature(iter_collect_into)]
mod elevation;
mod file_cache;

use elevation::{lookup_elevations, ElevationRequestBody};
use itertools::Itertools;
use log::{info, trace};
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::prelude::UnGraphMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::sync::Arc;
use std::{collections::HashMap, fs::create_dir_all};
use tokio::sync::Mutex;
use traits::{CollectTuples, IntoJoinConcurrently, IntoNodeIdPoint, ParMapCollect};

pub type NodeId = i64;
pub type Elevation = f64;
pub type Point = geo::Point<f64>;

pub type Points = HashMap<NodeId, Point>;
pub type Elevations = HashMap<NodeId, f64>;
pub type Edges = Vec<(NodeId, NodeId)>;

const OSM_INPUT_FILE: &str = "planet.osm.pbf";
const POINTS_FILE: &str = "data/points.postcard";
const EDGES_FILE: &str = "data/edges.postcard";
const ELEVATIONS_FILE: &str = "data/elevations.json";

fn create_elements() -> ElementReader<std::io::BufReader<std::fs::File>> {
    ElementReader::from_path(OSM_INPUT_FILE).unwrap()
}

fn read_postcard_data<T: serde::de::DeserializeOwned>(path: &str) -> std::io::Result<T> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let data: T = postcard::from_bytes(&buffer).unwrap();
    Ok(data)
}

fn create_graph() -> UnGraphMap<i64, ()> {
    if fs::exists(EDGES_FILE).unwrap() {
        info!("{EDGES_FILE} exists, reading");
        let edges = read_postcard_data::<Edges>(EDGES_FILE).unwrap();
        edges
            .into_iter()
            .map(|(left, right)| (left, right, ()))
            .collect()
    } else {
        info!("{EDGES_FILE} does not exist, creating");
        let graph = get_unweighted_cyclable_graphmap_from_elements(create_elements());
        let edges = graph
            .all_edges()
            .map(|(left, right, _)| (left, right))
            .collect_vec();
        let contents = postcard::to_stdvec(&edges).unwrap();
        fs::write(EDGES_FILE, contents.as_slice()).unwrap();
        graph
    }
}

fn create_points(graph: &UnGraphMap<NodeId, ()>) -> Points {
    if fs::exists(POINTS_FILE).unwrap() {
        info!("{POINTS_FILE} exists, reading");
        read_postcard_data::<Points>(POINTS_FILE).unwrap()
    } else {
        info!("{POINTS_FILE} does not exist, creating");
        let points = create_elements().par_map_collect(|element| {
            get_points_by_node_id(element, |node_id| graph.contains_node(*node_id))
        });
        let contents = postcard::to_stdvec(&points).unwrap();
        fs::write(POINTS_FILE, contents.as_slice()).unwrap();
        points
    }
}

// The pattern is "Read from file and if it doesn't exist then make it exist"
pub async fn get() {
    let client = reqwest::Client::new();

    create_dir_all("data").unwrap();

    // Read from files, otherwise create them.
    info!("Creating graph");
    let graph = create_graph();
    info!("Creating points");
    let points = create_points(&graph);
    info!("Creating elevations");
    let elevations = create_elevations(&client, &points).await;
}

// We need to write incrementally to the file as it receives data from the API,
// Because with a huge dataset in memory, the responses from the API are very very slow.
//
// Using JSON because it's easily to read & write stream as separated values.
// Will consider postcard COBS flavour but not yet.
async fn create_elevations(client: &reqwest::Client, nodes: &HashMap<i64, Point>) -> Elevations {
    const CONCURRENCY: usize = 16;
    const CHUNKS: usize = 1_000;

    let mut elevations_existing = if fs::exists(ELEVATIONS_FILE).unwrap() {
        info!("{ELEVATIONS_FILE} exists, reading");
        let contents = fs::read_to_string(ELEVATIONS_FILE).unwrap();
        serde_json::Deserializer::from_str(&contents)
            .into_iter::<Elevations>()
            .flat_map(|elevations| elevations.unwrap().into_iter())
            .collect::<Elevations>()
    } else {
        info!("{ELEVATIONS_FILE} does not exist, setting empty elevations");
        Elevations::default()
    };

    let total = nodes.len() - elevations_existing.len();
    info!("{total} total elevations");

    // Write to one file as each future completes.
    let writer = Arc::new(Mutex::new(File::create(ELEVATIONS_FILE).unwrap()));

    nodes
        .iter()
        // Only fetch the NodeId's we're missing.
        .filter(|(node_id, _)| !elevations_existing.contains_key(*node_id))
        .chunks(CHUNKS)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .enumerate()
        .map(|(index, (node_ids, points))| {
            let writer = writer.clone();

            async move {
                let response = lookup_elevations(&client, ElevationRequestBody::from_iter(points))
                    .await
                    .unwrap();
                let elevations = node_ids
                    .into_iter()
                    .zip_eq(response)
                    .collect::<Elevations>();
                let contents = serde_json::to_vec(&elevations).unwrap();
                info!("elevations: chunk {} of {}: fetching", index + 1, total);
                writer.lock().await.write_all(&contents.as_slice()).unwrap();
                info!("elevations: chunk {} of {}: complete", index + 1, total);
                elevations
            }
        })
        .join_concurrently::<Vec<_>>(CONCURRENCY)
        .await
        .into_iter()
        .flatten()
        .collect_into(&mut elevations_existing);

    elevations_existing
}

/// Creates a `HashMap` of points where `node_id`'s
fn get_points_by_node_id(
    element: Element<'_>,
    contains: impl Fn(&i64) -> bool,
) -> HashMap<i64, Point> {
    element
        .node_id_point()
        .filter(|(node_id, _)| contains(node_id))
        .map(|(node_id, point)| {
            let mut hashmap = HashMap::<i64, Point>::with_capacity(1);
            hashmap.insert(node_id, point);
            hashmap
        })
        .unwrap_or_default()
}

/// Creates an undirected, unweighted graph from all ways in an Open Street Maps PBF.
fn get_unweighted_cyclable_graphmap_from_elements<R>(
    elements: ElementReader<R>,
) -> UnGraphMap<i64, ()>
where
    R: Read + Send,
{
    elements
        .par_map_reduce(
            get_cyclable_node_ids_from_element,
            || UnGraphMap::default(),
            |mut accu, curr| {
                accu.extend(curr.all_edges());
                accu
            },
        )
        .unwrap()
}

/// Creates an undirected `GraphMap` when an element is a way.
fn get_cyclable_node_ids_from_element(element: Element<'_>) -> UnGraphMap<i64, ()> {
    match element {
        Element::Way(way) => Some(way),
        _ => None,
    }
    .filter(|way| contains_cycleable_tags(way.tags()))
    .map(|way| {
        way.refs()
            .tuple_windows::<(_, _)>()
            .map(|(from, to)| (from, to, ()))
            .collect::<UnGraphMap<_, _>>()
    })
    .unwrap_or_default()
}

/// Returns true when a combination of any tags indicate it is cyclable.
/// Inferred from https://wiki.openstreetmap.org/wiki/Map_features
fn contains_cycleable_tags(tags: TagIter<'_>) -> bool {
    let mut highway_footway = false;
    let mut bicycle_yes = false;

    for tag in tags {
        match tag {
            ("highway", "footway") => {
                highway_footway = true;
            }
            ("bicycle", "yes") => {
                bicycle_yes = true;
            }
            _ => {}
        }

        if highway_footway && bicycle_yes {
            return true;
        }

        if cyclable_way(tag) {
            return true;
        }
    }

    false
}

/// Returns true when a tag for a way is cyclable.
/// Inferred from https://wiki.openstreetmap.org/wiki/Map_features
fn cyclable_way(pair: (&str, &str)) -> bool {
    matches!(
        pair,
        (
            "highway",
            "trunk"
                | "primary"
                | "secondary"
                | "tertiary"
                | "residential"
                | "living_street"
                | "service"
                | "pedestrian"
                | "road"
                | "cycleway"
        ) | ("cycleway", _)
            | ("bicycle_road", "yes")
    )
}
