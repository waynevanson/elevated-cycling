use geo::{haversine_distance::HaversineDistance, Point};
use osmpbf::{Element, ElementReader, TagIter, WayRefIter};
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
    fn into_points_by_id_within_range(
        self,
        origin: &Point,
        radius_km: f64,
    ) -> osmpbf::Result<HashMap<NodeId, Point>>;
}

impl<R: Read + Send> IntoPointsByNodeId for ElementReader<R> {
    fn into_points_by_id_within_range(
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

pub trait IntoUndirectedGraph<I>
where
    Self: Iterator,
    I: Copy + Hash + Ord,
{
    fn into_undirected_graph_map(self) -> GraphMap<I, (), Undirected>;
}

// we could probably use (N,N,&E) as iterator item but this is intuitively easier.
impl<I, T> IntoUndirectedGraph<I> for T
where
    T: Iterator<Item = I>,
    I: Copy + Hash + Ord,
{
    fn into_undirected_graph_map(self) -> GraphMap<I, (), Undirected> {
        self.fold(
            (GraphMap::new(), None),
            |(mut graph_map, previous), curr_node_id| {
                if let Some(prev_node_id) = previous {
                    graph_map.add_edge(prev_node_id, curr_node_id, ());
                } else {
                    graph_map.add_node(curr_node_id);
                }

                return (graph_map, Some(curr_node_id));
            },
        )
        .0
    }
}

pub fn join_nodes_into_graph<R: Read + Send>(
    elements: ElementReader<R>,
    points_by_node_id: &HashMap<NodeId, Point>,
) -> osmpbf::Result<GraphMap<NodeId, (), Undirected>> {
    elements.par_map_reduce(
        |element| {
            match element {
                Element::Way(way) => Some(way),
                _ => None,
            }
            .filter(|way| way.tags().is_cyclable())
            .map(|way| {
                way.refs()
                    .filter(|way_node_id| points_by_node_id.contains_key(&way_node_id))
                    .into_undirected_graph_map()
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
