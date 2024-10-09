use indexmap::IndexSet;
use petgraph::{
    visit::{IntoNeighborsDirected, NodeCount},
    Direction::Outgoing,
};
use std::{hash::Hash, marker::PhantomData};

pub trait IntoAllSimplePaths
where
    Self: NodeCount + IntoNeighborsDirected,
    Self::NodeId: Eq + Hash,
{
    fn into_all_simple_paths<Collection>(
        self,
        from: Self::NodeId,
        to: Self::NodeId,
        min_intermediate_nodes: usize,
        max_intermediate_nodes: Option<usize>,
    ) -> AllSimplePathsIter<Self, Collection>
    where
        Collection: FromIterator<Self::NodeId>,
    {
        AllSimplePathsIter {
            graph: self,
            to,
            min_length: min_intermediate_nodes + 1,
            max_length: max_intermediate_nodes.map_or_else(|| self.node_count() - 1, |l| l + 1),
            visited: IndexSet::from_iter(Some(from)),
            stack: vec![self.neighbors_directed(from, Outgoing)],
            collection: PhantomData,
        }
    }
}

impl<T> IntoAllSimplePaths for T
where
    T: NodeCount + IntoNeighborsDirected,
    T::NodeId: Eq + Hash,
{
}

// I think I borrowed this from somewhere..
pub struct AllSimplePathsIter<Graph, Collection>
where
    Graph: NodeCount + IntoNeighborsDirected,
    Graph::NodeId: Eq + Hash,
{
    graph: Graph,
    to: Graph::NodeId,
    min_length: usize,
    // how many nodes are allowed in simple path up to target node
    // it is min/max allowed path length minus one, because it is more appropriate when implementing lookahead
    // than constantly add 1 to length of current path
    max_length: usize,
    // list of visited nodes
    visited: IndexSet<Graph::NodeId>,
    // list of childs of currently exploring path nodes,
    // last elem is list of childs of last visited node
    stack: Vec<Graph::NeighborsDirected>,
    collection: PhantomData<Collection>,
}

impl<'a, Graph, Collection> Iterator for AllSimplePathsIter<Graph, Collection>
where
    Graph: NodeCount + IntoNeighborsDirected,
    Graph::NodeId: Eq + Hash,
    Collection: FromIterator<Graph::NodeId>,
{
    type Item = Collection;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(children) = self.stack.last_mut() {
            if let Some(child) = children.next() {
                if self.visited.len() < self.max_length {
                    if child == self.to {
                        if self.visited.len() >= self.min_length {
                            let path = self
                                .visited
                                .iter()
                                .cloned()
                                .chain(Some(self.to))
                                .collect::<Collection>();
                            return Some(path);
                        }
                    } else if !self.visited.contains(&child) {
                        self.visited.insert(child);
                        self.stack
                            .push(self.graph.neighbors_directed(child, Outgoing));
                    }
                } else {
                    if (child == self.to || children.any(|v| v == self.to))
                        && self.visited.len() >= self.min_length
                    {
                        let path = self
                            .visited
                            .iter()
                            .cloned()
                            .chain(Some(self.to))
                            .collect::<Collection>();
                        return Some(path);
                    }
                    self.stack.pop();
                    self.visited.pop();
                }
            } else {
                self.stack.pop();
                self.visited.pop();
            }
        }
        None
    }
}
