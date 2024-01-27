mod algo;
mod elevation;

use crate::{
    algo::{join_nodes_into_graph, IntoPointsByNodeId},
    elevation::lookup_elevation,
};
use axum::{response::Json, routing::get, Router};
use clap::Parser;
use geo::Point;
use osmpbf::ElementReader;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
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
    coordinates: Vec<(f32, f32)>,
}

#[tokio::main]
async fn main() {
    let file_path = Args::parse().file_path;
    let client = Client::new();
    let create_elements = move || ElementReader::from_path(&file_path).unwrap();

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:example@database:5432")
        .await
        .unwrap();

    let handler = |Json(request): Json<CircuitDownHillRequest>| async move {
        let origin = Point::from((request.latitude, request.longitude));
        let nodes = create_elements()
            .into_points_by_id_within_range(&origin, request.max_radius)
            .unwrap();

        let graph = join_nodes_into_graph(create_elements(), &nodes).unwrap();

        let locations = graph
            .nodes()
            .filter_map(|node_id| nodes.get(&node_id))
            .collect::<Vec<_>>();

        let elevations = lookup_elevation(client, &locations).await;

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
