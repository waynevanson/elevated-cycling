use petgraph::prelude::UnGraphMap;

use crate::osm_pbf::NodeId;

/// Only retrieve enough data to construct a graph where paths interstect.
/// We don't only care about nodes in between when the path is really long.
///
/// Each intersection node containing n forks contains 1 + n forks.
pub fn connections(graph: UnGraphMap<NodeId, ()>) -> UnGraphMap<NodeId, ()> {
    let mut connections = UnGraphMap::default();

    for node in graph.nodes() {
        // if it's in the graph, it's probably a neighbour.
        if connections.contains_node(node) {
            continue;
        }

        // does it have two or less neightbours or more?
        let neighbours = graph.neighbors(node);

        let size = neighbours
            .size_hint()
            .1
            .expect("Expected to get the size of neighbours");

        // path node, skip
        if size <= 2 {
            continue;
        }

        connections.add_node(node);

        for neighbour in neighbours {
            connections.add_edge(node, neighbour, ());
        }
    }

    return connections;
}
