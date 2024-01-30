use axum::response::Html;
use axum_extra::extract::Query;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct MapParams {
    // Let's just return this thing exactly how it is.
    pub mapbbcode: String,
}

// /?mapbbcode=<map></map>
pub async fn map_handler(params: Query<MapParams>) -> Html<String> {
    let mapbbcode = &params.mapbbcode;

    let path = "/app/private/map.html";
    let html = fs::read(path).unwrap();
    let html = String::from_utf8_lossy(&html).replace("{MAPBBCODE}", &mapbbcode);

    Html(html)
}
