use anyhow::Result;
use itertools::Itertools;
use osmpbf::{reader::ElementReader, Element, TagIter};
use petgraph::prelude::{GraphMap, UnGraphMap};
use std::{fs::File, io::BufReader, path::Path};

/// Creates an undirected, unweighted graph from all ways in an Open Street Maps PBF.
pub fn get_unweighted_cyclable_graphmap_from_elements(path: &Path) -> Result<UnGraphMap<i64, ()>> {
    let pbf = ElementReader::new(BufReader::with_capacity(1024 * 1024, File::open(path)?));

    // Bulk inserts
    // https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-bind-an-array-to-a-values-clause-how-can-i-do-bulk-inserts
    let graph = pbf.par_map_reduce(
        get_cyclable_node_ids_from_element,
        || GraphMap::default(),
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
            .collect::<UnGraphMap<i64, ()>>()
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
