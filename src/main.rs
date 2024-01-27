mod elevation;
mod osm_pbf;

use crate::{
    elevation::{lookup_elevation, ElevationLocation, ElevationRequestBody},
    osm_pbf::{IntoCyclableNodes, IntoPointsByNodeId},
};
use axum::{response::Json, routing::get, Router};
use clap::Parser;
use geo::Point;
use itertools::Itertools;
use osmpbf::ElementReader;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

        let graph = create_elements()
            .into_cyclable_nodes(&points_by_node_id)
            .unwrap();

        let elevation_bodies = graph
            .nodes()
            .filter_map(|node_id| points_by_node_id.get(&node_id))
            .chunks(1_000)
            .into_iter()
            .map(|chunk| chunk.map(ElevationLocation::from).collect::<Vec<_>>())
            .map(ElevationRequestBody::from)
            .collect::<Vec<_>>();

        let _elevations = lookup_elevation(client, &elevation_bodies[0]).await;

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
