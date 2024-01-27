use geo::{haversine_distance::HaversineDistance, Point};
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::{graphmap::GraphMap, Undirected};
use std::{collections::HashMap, hash::Hash, io::Read};

type NodeId = i64;

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

pub trait IntoPointsByNodeId {
    fn into_points_by_node_id_within_range(
        self,
        origin: &Point,
        radius_km: f64,
    ) -> osmpbf::Result<HashMap<NodeId, Point>>;
}

impl<R: Read + Send> IntoPointsByNodeId for ElementReader<R> {
    fn into_points_by_node_id_within_range(
        self,
        origin: &Point,
        radius_km: f64,
    ) -> osmpbf::Result<HashMap<NodeId, Point>> {
        self.par_map_reduce(
            |element| {
                match element {
                    Element::Node(node) => Some((node.id(), Point::from((node.lat(), node.lon())))),
                    Element::DenseNode(node) => {
                        Some((node.id(), Point::from((node.lat(), node.lon()))))
                    }
                    _ => None,
                }
                .filter(|(_, coordinate)| coordinate.haversine_distance(origin) < radius_km)
                .map_or_else(
                    || HashMap::new(),
                    |coordinate| HashMap::from_iter([coordinate]),
                )
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
        points_by_node_id: &HashMap<NodeId, Point>,
    ) -> osmpbf::Result<GraphMap<NodeId, (), Undirected>>;
}

impl<R: Read + Send> IntoCyclableNodes for ElementReader<R> {
    fn into_cyclable_nodes(
        self,
        points_by_node_id: &HashMap<NodeId, Point>,
    ) -> osmpbf::Result<GraphMap<NodeId, (), Undirected>> {
        self.par_map_reduce(
            |element| {
                match element {
                    Element::Way(way) => Some(way),
                    _ => None,
                }
                .filter(|way| way.tags().is_cyclable())
                .map(|way| {
                    way.refs()
                        .filter(|way_node_id| points_by_node_id.contains_key(&way_node_id))
                        .scan(None, |state: &mut Option<NodeId>, item| {
                            if let Some(previous) = state {
                                Some((*previous, item, &()))
                            } else {
                                None
                            }
                        })
                        .collect::<GraphMap<NodeId, (), Undirected>>()
                })
                .unwrap_or_default()
            },
            || GraphMap::default(),
            |mut accu, curr| {
                accu.extend(curr.all_edges());
                accu
            },
        )
    }
}
