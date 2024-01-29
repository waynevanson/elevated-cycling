mod all_simple_paths;
mod elevation;
mod osm_pbf;
mod split_while;
mod tupled_joined;

use crate::{
    all_simple_paths::IntoAllSimplePaths,
    elevation::{lookup_elevations, ElevationRequestBody},
    osm_pbf::{IntoCyclableNodes, IntoPointsByNodeId, NodeId},
    split_while::IntoSplitWhile,
    tupled_joined::IntoTupleJoinedIter,
};
use axum::{response::Json, routing::get, Router};
use clap::Parser;
use futures::{
    future::{join_all, JoinAll},
    prelude::*,
};
use geo::Point;
use itertools::{FoldWhile, Itertools};
use osmpbf::ElementReader;
use petgraph::{graphmap::GraphMap, Directed};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Parser)]
struct Args {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct CircuitDownHillRequest {
    latitude: f64,
    longitude: f64,
    max_radius: f64,
}

#[derive(Debug, Clone, Serialize)]
struct CircuitDownHillResponse {
    coordinates: Vec<(f64, f64)>,
}

trait CollectTuples<A, B> {
    fn collect_tuples<Left, Right>(self) -> (Left, Right)
    where
        Left: Default + Extend<A>,
        Right: Default + Extend<B>;
}

impl<T, A, B> CollectTuples<A, B> for T
where
    T: Iterator<Item = (A, B)>,
{
    fn collect_tuples<Left, Right>(self) -> (Left, Right)
    where
        Left: Default + Extend<A>,
        Right: Default + Extend<B>,
    {
        let mut left = Left::default();
        let mut right = Right::default();

        for (left_item, right_item) in self {
            left.extend(Some(left_item));
            right.extend(Some(right_item));
        }

        (left, right)
    }
}

trait IntoJoinAll: IntoIterator + Sized {
    fn futures_join_all(self) -> JoinAll<Self::Item>
    where
        Self: IntoIterator,
        Self::Item: Future,
    {
        join_all(self)
    }
}

impl<T> IntoJoinAll for T where T: IntoIterator + Sized {}

type Gradient = f64;
type Elevation = f64;

#[tokio::main]
async fn main() {
    let file_path = Args::parse().file_path;
    let client = Client::new();
    let create_elements = move || ElementReader::from_path(&file_path).unwrap();

    let handler = |Json(request): Json<CircuitDownHillRequest>| async move {
        let origin = Point::from((request.latitude, request.longitude));
        let points_by_node_id = create_elements()
            .into_points_by_node_id_within_range(&origin, request.max_radius)
            .unwrap();

        let node_id_origin = points_by_node_id
            .iter()
            .map(|(node_id, (_, distance))| (node_id, distance))
            .fold_while(
                None,
                |closest: Option<(NodeId, &f64)>, (node_id, distance)| {
                    if *distance == 0.0 {
                        FoldWhile::Done(Some((*node_id, distance)))
                    } else if let Some((node_id_closest, distance_closest)) = closest {
                        let closest = if distance < distance_closest {
                            (*node_id, distance)
                        } else {
                            (node_id_closest, distance_closest)
                        };

                        FoldWhile::Continue(Some(closest))
                    } else {
                        FoldWhile::Continue(Some((*node_id, distance)))
                    }
                },
            )
            .into_inner()
            .expect("Could not find an origin node_id")
            .0;

        let graph_node_ids = create_elements()
            .into_cyclable_nodes(&points_by_node_id)
            .unwrap();

        let elevation_by_node_id: HashMap<NodeId, Elevation> = graph_node_ids
            .nodes()
            .filter_map(|node_id| {
                points_by_node_id
                    .get(&node_id)
                    .map(|point_distance| (node_id, point_distance.0))
            })
            .chunks(1_000)
            .into_iter()
            .map(|chunk| chunk.collect_tuples::<Vec<NodeId>, Vec<_>>())
            .map(|(node_ids, points)| {
                lookup_elevations(&client, ElevationRequestBody::from_iter(points)).map(
                    |elevations| {
                        node_ids
                            .into_iter()
                            .zip_eq(elevations)
                            .collect::<HashMap<NodeId, Elevation>>()
                    },
                )
            })
            .futures_join_all()
            .await
            .into_iter()
            .flatten()
            .collect();

        let graph_gradient: GraphMap<NodeId, Gradient, Directed> = graph_node_ids
            .all_edges()
            .map(|(from, to, _)| {
                let from_elevation = elevation_by_node_id.get(&from).unwrap();
                let to_elevation = elevation_by_node_id.get(&to).unwrap();
                let gradient = from_elevation / to_elevation;
                (from, to, gradient)
            })
            .flat_map(|(from, to, gradient)| [(from, to, gradient), (to, from, -gradient)])
            .collect();

        let paths = graph_gradient
            .into_all_simple_paths::<Vec<_>>(node_id_origin, node_id_origin, 0, None)
            .map(|mut path| {
                path.push(node_id_origin);
                path
            });

        // HashMap<Vec<NodeId>, Score>
        // find highest score
        // maybe chunk into up/downhills and their reward size.
        // score chunks higher if uphill + short or downhilll + long

        let path = paths.into_iter().map(|path| {
            path.tuple_joined()
                .flat_map(|(from, to)| graph_gradient.edge_weight(from, to))
                .copied()
                .split_while(
                    || Vec::<f64>::new(),
                    |mut splits, gradient| {
                        if let Some(last) = splits.last().copied() {
                            if (last > 0.0) == (gradient > 0.0) {
                                splits.push(gradient);
                                FoldWhile::Continue(splits)
                            } else {
                                FoldWhile::Done(splits)
                            }
                        } else {
                            splits.push(gradient);
                            FoldWhile::Continue(splits)
                        }
                    },
                )
                .map(|splits| {
                    let size = splits.len() as f64;
                    let sum = splits.into_iter().sum::<f64>();
                    let average = sum / size;
                    // we only know the reward depending on how far in we are.
                    if average > 0.0 {
                        // reward for uphill
                    } else {
                        // reward for downhill
                    }
                })
        });

        // After we figure out what a good route is, return the points

        Json(CircuitDownHillResponse {
            coordinates: vec![],
        })
    };

    // build our application with a route
    let app = Router::new().route("/", get(handler));

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
