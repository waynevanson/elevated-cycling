use crate::traits::{IntoNodeIdPoint, ParMapCollect};
use geo::{Distance, Haversine, Point};
use itertools::Itertools;
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::prelude::UnGraphMap;
use std::{collections::HashMap, io::Read};

#[derive(Debug, Clone)]
pub struct Buffer {
    pub points: HashMap<i64, Point<f64>>,
    pub distances: UnGraphMap<i64, f64>,
}

/// Creates a buffer of values used for all requests.
pub async fn create_buffer() -> Buffer {
    let create_elements = || ElementReader::from_path("./planet.osm.pbf").unwrap();

    let nodes = get_unweighted_cyclable_graphmap_from_elements(create_elements());

    let points = create_elements().par_map_collect(|element| {
        get_points_by_node_id(element, |node_id| nodes.contains_node(*node_id))
    });

    let distances = get_distances_from_nodes(nodes, &points);

    Buffer { points, distances }
}

fn get_distances_from_nodes(
    graph: UnGraphMap<i64, ()>,
    points: &HashMap<i64, Point<f64>>,
) -> UnGraphMap<i64, f64> {
    graph
        .all_edges()
        .filter_map(|(from, to, _)| {
            let left = points.get(&from)?;
            let right = points.get(&to)?;
            let distance_difference = Haversine::distance(*left, *right);
            Some((from, to, distance_difference))
        })
        .collect()
}

/// Creates a `HashMap` of points where `node_id`'s
fn get_points_by_node_id(
    element: Element<'_>,
    contains: impl Fn(&i64) -> bool,
) -> HashMap<i64, Point<f64>> {
    element
        .node_id_point()
        .filter(|(node_id, _)| contains(node_id))
        .map(|(node_id, point)| {
            let mut hashmap = HashMap::<i64, Point<f64>>::with_capacity(1);
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
