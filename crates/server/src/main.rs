// use nix to package the docker file
// use a nix command to run docjer compose with the image we want.
#![feature(let_chains)]

mod elevation;
mod traits;

use elevation::{lookup_elevations, ElevationRequestBody};
use geo::Point;
use itertools::Itertools;
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::prelude::UnGraphMap;
use std::{collections::HashMap, io::Read};
use traits::{CollectTuples, IntoJoinConcurrently, ParMapCollect, PartitionResults};

#[tokio::main]
async fn main() {
    let reqwest_client = reqwest::Client::new();

    let points = get_points();
    let _elevations = get_elevation_by_node_id(&reqwest_client, &points).await;
}

async fn get_elevation_by_node_id<'a>(
    client: &reqwest::Client,
    nodes: &HashMap<i64, Point<f64>>,
) -> HashMap<i64, f64> {
    nodes
        .into_iter()
        .chunks(1_000)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .map(|(node_ids, points)| async {
            lookup_elevations(&client, ElevationRequestBody::from_iter(points))
                .await
                .map(|elevations| {
                    node_ids
                        .into_iter()
                        .zip_eq(elevations)
                        .collect::<HashMap<i64, f64>>()
                })
        })
        .join_concurrently::<Vec<_>>(4)
        .await
        .into_iter()
        .partition_results::<Vec<_>, Vec<_>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect()
}

fn get_points() -> HashMap<i64, Point> {
    let create_elements = || ElementReader::from_path("./planet.osm.pbf").unwrap();

    let cyclable_node_ids = get_unweighted_cyclable_graphmap_from_elements(create_elements());

    let points = create_elements().par_map_collect(|element| {
        get_points_by_node_id(element, |node_id| cyclable_node_ids.contains_node(*node_id))
    });

    points
}

fn get_points_by_node_id(
    element: Element<'_>,
    contains: impl Fn(&i64) -> bool,
) -> HashMap<i64, Point<f64>> {
    match element {
        Element::Node(node) => Some((node.id(), (node.lat(), node.lon()))),
        Element::DenseNode(node) => Some((node.id(), (node.lat(), node.lon()))),
        _ => None,
    }
    .filter(|(node_id, _)| contains(node_id))
    .map(|(node_id, lat_lon)| {
        let mut hashmap = HashMap::<i64, Point<f64>>::with_capacity(1);
        let point = Point::from(lat_lon);
        hashmap.insert(node_id, point);
        hashmap
    })
    .unwrap_or_default()
}

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
    )
}
