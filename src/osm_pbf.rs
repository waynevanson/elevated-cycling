use geo::haversine_distance::HaversineDistance;
use ordered_float::OrderedFloat;
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::{graphmap::GraphMap, prelude::UnGraphMap, Undirected};
use std::{collections::HashMap, io::Read};

use crate::{Distance, Point};

pub type NodeId = i64;

pub trait IsCyclable {
    fn is_cyclable(self) -> bool;
}

impl IsCyclable for TagIter<'_> {
    fn is_cyclable(self) -> bool {
        let mut highway_footway = false;
        let mut bicycle_yes = false;

        for tag in self {
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

type Meters = f64;

pub trait IntoPointsByNodeId {
    fn into_points_by_node_id_within_range(
        self,
        origin: &Point,
        radius_km: &OrderedFloat<f64>,
    ) -> osmpbf::Result<HashMap<NodeId, (Point, Distance)>>;
}

impl<R: Read + Send> IntoPointsByNodeId for ElementReader<R> {
    fn into_points_by_node_id_within_range(
        self,
        origin: &Point,
        radius_km: &OrderedFloat<f64>,
    ) -> osmpbf::Result<HashMap<NodeId, (Point, Distance)>> {
        self.par_map_reduce(
            |element| {
                match element {
                    Element::Node(node) => Some((
                        node.id(),
                        Point::from((OrderedFloat(node.lat()), OrderedFloat(node.lon()))),
                    )),
                    Element::DenseNode(node) => Some((
                        node.id(),
                        Point::from((OrderedFloat(node.lat()), OrderedFloat(node.lon()))),
                    )),
                    _ => None,
                }
                .map(|(node_id, point)| (node_id, (point, point.haversine_distance(&origin))))
                .filter(|(_, (_, distance))| distance <= radius_km)
                .map(|entry| HashMap::from_iter([entry]))
                .unwrap_or_default()
            },
            || HashMap::new(),
            |mut accu, curr| {
                accu.extend(curr);
                accu
            },
        )
    }
}

pub trait IntoCyclableNodes {
    fn into_cyclable_nodes(
        self,
        points_by_node_id: &HashMap<NodeId, (Point, Distance)>,
    ) -> osmpbf::Result<UnGraphMap<NodeId, Distance>>;
}

impl<R: Read + Send> IntoCyclableNodes for ElementReader<R> {
    fn into_cyclable_nodes(
        self,
        points_by_node_id: &HashMap<NodeId, (Point, Distance)>,
    ) -> osmpbf::Result<UnGraphMap<NodeId, Distance>> {
        self.par_map_reduce(
            |element| {
                match element {
                    Element::Way(way) => Some(way),
                    _ => None,
                }
                .filter(|way| way.tags().is_cyclable())
                .into_iter()
                .flat_map(|way| way.refs())
                .filter(|way_node_id| points_by_node_id.contains_key(&way_node_id))
                // create edges between nodes, calculating the distance too.
                .scan(None, |state: &mut Option<NodeId>, node_id_to| {
                    let item = state.map(|node_id_from| {
                        let from = points_by_node_id.get(&node_id_from).unwrap().0;
                        let to = points_by_node_id.get(&node_id_to).unwrap().0;
                        let distance = from.haversine_distance(&to);
                        (node_id_from, node_id_to, distance)
                    });
                    *state = Some(node_id_to);
                    Some(item)
                })
                .flatten()
                .collect::<UnGraphMap<NodeId, Distance>>()
            },
            || GraphMap::default(),
            |mut accu, curr| {
                accu.extend(curr.all_edges());
                accu
            },
        )
    }
}
