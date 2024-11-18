use crate::Point;
use anyhow::Result;
use petgraph::prelude::DiGraphMap;
use sqlx::postgres::*;
use sqlx::prelude::*;
use std::collections::HashMap;

#[derive(Debug, FromRow)]
pub struct NodeRow {
    id: i64,
    x: f64,
    y: f64,
    z: Option<f64>,
}

#[derive(Debug, FromRow)]
pub struct EdgeRow {
    origin: i64,
    destination: i64,
}

// does the graph impl we looked at online for postgres do edges as separate rows?
// how about goe postgres?

pub async fn get_nodes_from_db(pool: &PgPool) -> Result<HashMap<i64, (Point, Option<f64>)>> {
    let sql = "SELECT (id, x, y) FROM nodes WHERE z IS NULL";
    let query = sqlx::query_as::<_, NodeRow>(sql);
    let nodes = query.fetch_all(pool).await?;

    let points = nodes
        .into_iter()
        .map(|row| (row.id, (Point::from((row.x, row.y)), row.z)))
        .collect();

    Ok(points)
}

// pub async fn get_graph_from_db(pool: &PgPool) -> Result<DiGraphMap<i64, (f64, Option<f64>)>> {
//     let sql = "SELECT origin, destination, gradient, distance FROM nodes";
//     let query = sqlx::query_as::<_, EdgeRow>(sql);
//     let edges = query.fetch_all(pool).await?;

//     let graph = edges
//         .into_iter()
//         .map(|row| (row.origin, row.destination, (row.distance, row.gradient)))
//         .collect();

//     Ok(graph)
// }
