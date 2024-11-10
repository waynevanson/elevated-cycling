mod handler;

use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use bootstrap_buffer::create_buffer;
use handler::{handler, HandlerState};
use liquid::ParserBuilder;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn create_state() -> HandlerState {
    let contents = include_str!("../templates/index.liquid");

    HandlerState {
        buffer: Arc::new(create_buffer().await),
        client: Arc::new(reqwest::Client::new()),
        template: Arc::new(
            ParserBuilder::with_stdlib()
                .build()
                .unwrap()
                .parse(&contents)
                .unwrap(),
        ),
    }
}

#[tokio::main]
async fn main() {
    println!("Bootstrapping server...");
    let contents = include_str!("../../../public/index.html");

    let state = create_state().await;
    let app = Router::new()
        .route("/:latitude/:longitude/:radius", get(handler))
        .with_state(state)
        .route("/", get(Html(contents)));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Listening on ${:?}", listener.local_addr());

    axum::serve(listener, app).await.unwrap();
}
