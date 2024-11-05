#![feature(let_chains)]

mod bootstrap_buffer;
mod elevation;
mod traits;

use bootstrap_buffer::create_buffer;
use elevation::{lookup_elevations, ElevationRequestBody};
use geo::{Distance, Haversine, Point};
use itertools::Itertools;
use petgraph::prelude::DiGraphMap;
use std::collections::HashMap;
use traits::{CollectTuples, IntoJoinConcurrently, PartitionResults};

#[tokio::main]
async fn main() {
    // per server
    let buffer = create_buffer().await;
    let client = reqwest::Client::new();

    // per request
    // todo - parameterise
    let origin: Point<f64> = Point::from((-38.032603, 145.335817));
    let radius: f64 = 5_000.0;

    // derivations
    let points: HashMap<&i64, &Point<f64>> = buffer
        .points
        .iter()
        .filter(|(_, point)| Haversine::distance(origin, **point) < radius)
        .collect();

    // buffering this API looks like it will take about 24 hours,
    // better to call it when we need it.
    let elevations = get_elevations_by_node_id(&client, &points).await;

    let _gradients: DiGraphMap<i64, f64> = buffer
        .distances
        .all_edges()
        .flat_map(|(from, to, distance)| {
            let left = elevations.get(&from)?;
            let right = elevations.get(&to)?;
            let gradient = (left - right) / distance;
            Some((from, to, gradient))
        })
        .flat_map(|(left, right, gradient)| [(left, right, gradient), (right, left, -gradient)])
        .collect();

    // now the path finding magic.

    // combine gradients, keeping nodes at elevations and when gradients go between - and +.
    // find highest elevation point
    // find paths to this point.
    // way up should be that with highest gradient.
    // way down should be that with lowest gradient.
}

/// I would love to be able to read directly from a file in rust but that's not
/// going to happen unless I put more time aside.
async fn get_elevations_by_node_id<'a>(
    client: &reqwest::Client,
    nodes: &HashMap<&'a i64, &Point<f64>>,
) -> HashMap<&'a i64, f64> {
    let concurrency = 32;
    let chunks = 100;
    let total = nodes.len() / chunks;

    nodes
        .iter()
        .map(|(node_id, value)| (*node_id, *value))
        .chunks(chunks)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .enumerate()
        .map(|(index, (node_ids, points))| async move {
            lookup_elevations(&client, ElevationRequestBody::from_iter(points))
                .await
                .map(|elevations| {
                    node_ids
                        .into_iter()
                        .zip_eq(elevations)
                        .collect::<HashMap<&i64, f64>>()
                })
                .map(|result| {
                    let position = index + 1;
                    println!("{position} of {total}");

                    result
                })
        })
        .join_concurrently::<Vec<_>>(concurrency)
        .await
        .into_iter()
        .partition_results::<Vec<_>, Vec<_>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect()
}
