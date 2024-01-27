use geo::{haversine_distance::HaversineDistance, Point};
use osmpbf::{Element, ElementReader, TagIter};
use petgraph::{graphmap::GraphMap, Undirected};
use std::{collections::HashMap, io::Read, iter::Inspect};

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
                .filter(|(_, coordinate)| {
                    coordinate.haversine_distance(origin) < radius_km * 1000.0
                })
                .map(|coordinate| HashMap::from_iter([coordinate]))
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
                .into_iter()
                .flat_map(|way| way.refs())
                .filter(|way_node_id| points_by_node_id.contains_key(&way_node_id))
                .scan(None, |state: &mut Option<NodeId>, node_id_to| {
                    let item = state.map(|node_id_from| (node_id_from, node_id_to, ()));
                    *state = Some(node_id_to);
                    Some(item)
                })
                .flatten()
                .collect::<GraphMap<NodeId, (), Undirected>>()
            },
            || GraphMap::default(),
            |mut accu, curr| {
                accu.extend(curr.all_edges());
                accu
            },
        )
    }
}

#[cfg(test)]
mod test {
    use crate::osm_pbf::IntoCyclableNodes;

    use super::IntoPointsByNodeId;
    use geo::Point;
    use itertools::Itertools;
    use osmpbf::ElementReader;
    use std::collections::HashMap;

    #[test]
    fn should_have_some_nodes_available() {
        let origin = Point::from((-38.03073, 145.32790));
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/map.osm.pbf");
        let result = ElementReader::from_path(path)
            .unwrap()
            .into_points_by_node_id_within_range(&origin, 10.0)
            .unwrap();

        assert_ne!(result, HashMap::new());
    }

    #[test]
    fn should_create_graph() {
        let origin = Point::from((-38.03073, 145.32790));
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/map.osm.pbf");
        let point_by_node_id = ElementReader::from_path(path)
            .unwrap()
            .into_points_by_node_id_within_range(&origin, 10.0)
            .unwrap();

        let result = ElementReader::from_path(path)
            .unwrap()
            .into_cyclable_nodes(&point_by_node_id)
            .unwrap()
            .nodes()
            .collect_vec();

        assert_ne!(result, Vec::<i64>::new());
    }
}
