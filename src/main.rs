use axum::{handler::Handler, response::Json, routing::get, Router};
use clap::Parser;
use osmpbf::ElementReader;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Get the file path at startup via CLI invocation
#[derive(Debug, Clone, Parser)]
struct Args {
    file_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct CircuitDownHillRequest {
    latitude: f32,
    longitude: f32,
    radius: u8,
}

#[derive(Debug, Clone, Serialize)]
struct CircuitDownHillResponse {
    coordinates: Vec<(f32, f32)>,
}

#[tokio::main]
async fn main() {
    let file_path = Args::parse().file_path;

    // build our application with a route
    let app = Router::new().route(
        "/",
        get(|Json(request): Json<CircuitDownHillRequest>| async {
            let reader = ElementReader::from_path(file_path).unwrap();
            Json(CircuitDownHillResponse {
                coordinates: vec![],
            })
        }),
    );

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}
