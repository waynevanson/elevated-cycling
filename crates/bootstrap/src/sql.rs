use std::collections::HashMap;

use anyhow::Result;
use petgraph::prelude::DiGraphMap;
use sqlx::prelude::*;

use crate::Point;

#[derive(Debug, FromRow)]
pub struct NodeRow {
    node_id: i64,
    elevation: f64,
    x: f64,
    y: f64,
}

#[derive(Debug, FromRow)]
pub struct EdgeRow {
    origin: i64,
    destination: i64,
    gradient: f64,
    distance: f64,
}

pub async fn yaow() -> Result<()> {
    let url = "postgres://user:password@localhost:5432/elevated-cycling";
    let pool = sqlx::postgres::PgPool::connect(url).await?;

    // read nodes for lookup
    let sql = "SELECT node_id, elevation, x, y FROM nodes";
    let query = sqlx::query_as::<_, NodeRow>(sql);
    let nodes = query.fetch_all(&pool).await?;
    let nodes: HashMap<_, _> = nodes
        .into_iter()
        .map(|row| (row.node_id, (Point::from((row.x, row.y)), row.elevation)))
        .collect();

    // read edges for graph
    let sql = "SELECT origin, destination, gradient, distance FROM nodes";
    let query = sqlx::query_as::<_, EdgeRow>(sql);
    let edges = query.fetch_all(&pool).await?;
    let edges: DiGraphMap<_, _> = edges
        .into_iter()
        .map(|row| (row.origin, row.destination, (row.distance, row.gradient)))
        .collect();

    Ok(())
}
