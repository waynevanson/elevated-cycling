use crate::bootstrap_buffer::Buffer;
use crate::elevation::{lookup_elevations, ElevationRequestBody};
use crate::traits::{Average, CollectTuples, IntoJoinConcurrently, PartitionResults};
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use geo::{Distance, Haversine, Point};
use itertools::Itertools;
use liquid::Template;
use petgraph::{
    algo::astar,
    prelude::{DiGraphMap, UnGraphMap},
};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct HandlerPathParams {
    latitude: f64,
    longitude: f64,
    radius: f64,
}

#[derive(Clone)]
pub struct HandlerState {
    pub buffer: Arc<Buffer>,
    pub client: Arc<Client>,
    pub template: Arc<Template>,
}

struct MapBBCode {}

pub struct HandlerResponse(Html<String>);

impl IntoResponse for HandlerResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, self.0.into_response()).into_response()
    }
}

#[axum::debug_handler]
pub async fn handler(
    State(state): State<HandlerState>,
    Path(params): Path<HandlerPathParams>,
) -> HandlerResponse {
    println!("{params:?}");
    // per server

    // per request
    // todo - parameterise
    let origin: Point<f64> = Point::from((params.latitude, params.longitude));

    // derivations
    let points = points_within_range(&state.buffer.points, &origin, params.radius);

    // buffering this API looks like it will take about 24 hours,
    // better to call it when we need it.
    let elevations = get_elevations_by_node_id(&state.client, points.clone()).await;

    let gradients = create_gradients(&state.buffer.distances, &points, &elevations);

    // now the path finding magic.

    // combine gradients, keeping nodes at elevations and when gradients go between - and +.
    // find highest elevation point
    // find paths to this point.
    // way up should be that with highest gradient.
    // way down should be that with lowest gradient.

    let (closest_node_id, closest_point) = find_closest(&points, &origin);

    let (highest_node_id, highest_point) = find_highest(&elevations);

    let travelling_to_path = find_shortest_path(&gradients, &closest_node_id, &highest_node_id);

    println!("Way up {:?}", travelling_to_path);

    let travelling_to_gradient = travelling_to_path
        .iter()
        .tuple_windows::<(_, _)>()
        .map(|(&from, &to)| gradients.edge_weight(from, to).unwrap())
        .average::<f64>();

    // Avoid using same path where possible
    let (travelling_to_cost, travelling_to_path) = astar(
        &gradients,
        *highest_node_id,
        |finish| finish == closest_node_id,
        |(from, to, gradient)| {
            if gradient < &0. {
                10.
            } else {
                *gradient
            }
        },
        |node_id| 0.,
    )
    .unwrap();
    println!("way down {:?}", travelling_to_path);

    let mapbbcode = travelling_to_path
        .into_iter()
        .filter_map(|node_id| points.get(&node_id))
        .map(|point| {
            [
                point.x().to_string(),
                ",".to_string(),
                point.y().to_string(),
            ]
            .into_iter()
            .collect::<String>()
        })
        .intersperse(" ".to_string())
        .collect::<String>();

    let mapbbcode = ["[map]", &mapbbcode, "[/map]"]
        .into_iter()
        .collect::<String>();

    let globals = liquid::object!({
        "mapbbcode": mapbbcode
    });

    let html = state.template.render(&globals).unwrap();

    HandlerResponse(Html(html))
}

fn points_within_range(
    points: &HashMap<i64, Point<f64>>,
    origin: &Point<f64>,
    radius: f64,
) -> HashMap<i64, Point<f64>> {
    points
        .iter()
        .filter(|(_, point)| Haversine::distance(*origin, **point) < radius)
        .map(|(node_id, point)| (*node_id, *point))
        .collect()
}

fn find_shortest_path(
    gradients: &DiGraphMap<i64, f64>,
    closest_node_id: &i64,
    highest_node_id: &i64,
) -> Vec<i64> {
    astar(
        &gradients,
        *closest_node_id,
        |finish| &finish == highest_node_id,
        |(from, to, gradient)| {
            if gradient < &0. {
                10.
            } else {
                *gradient
            }
        },
        |node_id| 0.,
    )
    .unwrap()
    .1
}

fn create_gradients(
    distances: &UnGraphMap<i64, f64>,
    points: &HashMap<i64, Point<f64>>,
    elevations: &HashMap<i64, f64>,
) -> DiGraphMap<i64, f64> {
    distances
        .all_edges()
        .filter(|(from, to, _)| points.contains_key(from) && points.contains_key(to))
        .flat_map(|(from, to, distance)| {
            let left = elevations.get(&from)?;
            let right = elevations.get(&to)?;
            let gradient = (left - right) / distance;
            Some((from, to, gradient))
        })
        .flat_map(|(left, right, gradient)| [(left, right, gradient), (right, left, -gradient)])
        .collect()
}

fn find_highest<'b>(elevations: &'b HashMap<i64, f64>) -> (&'b i64, &'b f64) {
    elevations
        .iter()
        .map(|(node_id, elevation)| (node_id, elevation))
        .reduce(
            |(left_node_id, left_elevation), (right_node_id, right_elevation)| {
                if left_elevation > right_elevation {
                    (left_node_id, left_elevation)
                } else {
                    (right_node_id, right_elevation)
                }
            },
        )
        .unwrap()
}

fn find_closest(points: &HashMap<i64, Point>, origin: &Point<f64>) -> (i64, f64) {
    points
        .iter()
        .map(|(&node_id, &point)| (node_id, Haversine::distance(*origin, point)))
        .reduce(
            |(left_node_id, left_distance), (right_node_id, right_distance)| {
                if left_distance < right_distance {
                    (left_node_id, left_distance)
                } else {
                    (right_node_id, right_distance)
                }
            },
        )
        .unwrap()
}

/// I would love to be able to read directly from a file in rust but that's not
/// going to happen unless I put more time aside.
async fn get_elevations_by_node_id(
    client: &reqwest::Client,
    nodes: HashMap<i64, Point<f64>>,
) -> HashMap<i64, f64> {
    const CONCURRENCY: usize = 32;
    const CHUNKS: usize = 100;
    let total = nodes.len() / CHUNKS;

    nodes
        .into_iter()
        .chunks(CHUNKS)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .collect_vec()
        .into_iter()
        .enumerate()
        .map(|(index, (node_ids, points))| async move {
            lookup_elevations(&client, ElevationRequestBody::from_iter(points))
                .await
                .map(|elevations| {
                    node_ids
                        .into_iter()
                        .zip_eq(elevations)
                        .collect::<HashMap<i64, f64>>()
                })
                .map(|result| {
                    let position = index + 1;
                    println!("{position} of {total}");

                    result
                })
        })
        .join_concurrently::<Vec<_>>(CONCURRENCY)
        .await
        .into_iter()
        .partition_results::<Vec<_>, Vec<_>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect()
}
