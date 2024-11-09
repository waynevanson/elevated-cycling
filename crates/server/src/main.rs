#![feature(let_chains)]

mod bootstrap_buffer;
mod elevation;
mod handler;
mod traits;

use axum::{routing::get, Router};
use bootstrap_buffer::create_buffer;
use handler::{handler, HandlerState};
use liquid::ParserBuilder;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn create_state() -> HandlerState {
    HandlerState {
        buffer: Arc::new(create_buffer().await),
        client: Arc::new(reqwest::Client::new()),
        template: Arc::new(
            ParserBuilder::with_stdlib()
                .build()
                .unwrap()
                .parse_file("./crates/server/templates/index.liquid")
                .unwrap(),
        ),
    }
}

#[tokio::main]
async fn main() {
    println!("Bootstrapping server...");

    let state = create_state().await;
    let app = Router::new()
        .route("/:latitude/:longitude/:radius", get(handler))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Listening on ${:?}", listener.local_addr());

    axum::serve(listener, app).await.unwrap();
}
