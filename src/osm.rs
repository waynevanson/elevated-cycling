use crate::traits::{ElementReaderExt, IntoNodeIdPoint, ParMapCollect};
use anyhow::Result;
use geo::Coord;
use itertools::Itertools;
use log::debug;
use osmpbf::{reader::ElementReader, Element, TagIter};
use petgraph::prelude::UnGraphMap;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

const READ_BUF_CAPACITY: usize = 8usize.pow(8);

pub fn derive_coords_from_osm_pbf(
    path: &Path,
    nodes: &HashSet<i64>,
) -> Result<HashMap<i64, Coord>> {
    let pbf = ElementReader::with_capacity(READ_BUF_CAPACITY, path)?;

    debug!("Extracting data from {:?} into memory", path);
    // ~31 seconds

    let coords = pbf.par_map_collect(|element| {
        let mut map = HashMap::with_capacity(1);
        map.extend(
            element
                // todo: filter for nodes first
                .node_id_point()
                .filter(|node_id| nodes.contains(&node_id.0)),
        );
        map
    });

    debug!("Extracted data from {:?} into memory", path);

    Ok(coords)
}

/// Creates an undirected, unweighted graph from all ways in an Open Street Maps PBF.
pub fn get_unweighted_cyclable_graphmap_from_elements(path: &Path) -> Result<UnGraphMap<i64, ()>> {
    let pbf = ElementReader::with_capacity(READ_BUF_CAPACITY, path)?;

    let graph = pbf.par_map_reduce(
        get_cyclable_node_ids_from_element,
        || UnGraphMap::default(),
        |mut accu, curr| {
            accu.extend(curr.all_edges());
            accu
        },
    )?;

    Ok(graph)
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
