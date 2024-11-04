// use nix to package the docker file
// use a nix command to run docjer compose with the image we want.
#![feature(let_chains)]

mod collect_tuples;
mod elevation;
mod futures_concurently;
mod partition_results;

use collect_tuples::CollectTuples;
use elevation::{lookup_elevations, ElevationRequestBody};
use futures_concurently::IntoJoinConcurrently;
use geo::Point;
use itertools::Itertools;
use osmpbf::{Element, ElementReader, TagIter};
use partition_results::PartitionResults;
use redis::{Commands, Connection};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
};

#[tokio::main]
async fn main() {
    let redis_client = redis::Client::open("redis://127.0.0.1:6379").unwrap();
    let mut redis_connection = redis_client.get_connection().unwrap();

    let reqwest_client = reqwest::Client::new();

    let points = get_points();
    let _elevations = get_elevation_by_node_id(&reqwest_client, &points).await;

    add_points_to_redis(&mut redis_connection, points);
}

fn add_points_to_redis(connection: &mut Connection, points: HashMap<i64, Point<f64>>) {
    let members = points
        .into_iter()
        .map(|(node_id, point)| (point.x(), point.y(), node_id))
        .collect::<Vec<_>>();

    connection.geo_add::<_, _, ()>("osm", members).unwrap();
}

async fn get_elevation_by_node_id<'a>(
    client: &reqwest::Client,
    nodes: &HashMap<i64, Point<f64>>,
) -> HashMap<i64, f64> {
    nodes
        .into_iter()
        .chunks(1_000)
        .into_iter()
        .map(|chunk| chunk.collect_tuples::<Vec<_>, Vec<_>>())
        .map(|(node_ids, points)| async {
            lookup_elevations(&client, ElevationRequestBody::from_iter(points))
                .await
                .map(|elevations| {
                    node_ids
                        .into_iter()
                        .zip_eq(elevations)
                        .collect::<HashMap<i64, f64>>()
                })
        })
        .join_concurrently::<Vec<_>>(4)
        .await
        .into_iter()
        .partition_results::<Vec<_>, Vec<_>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect()
}

// use redis for finding distances because cbf writing that logic again.
fn get_points() -> HashMap<i64, Point> {
    let create_elements = || ElementReader::from_path("./planet.osm.pbf").unwrap();

    let cyclable_node_ids = create_elements().par_map_collect(get_cyclable_node_ids_from_element);

    let points = create_elements()
        .par_map_collect(|element| get_points_by_node_id(element, &cyclable_node_ids));

    points
}

fn get_points_by_node_id(
    element: Element<'_>,
    node_ids: &HashSet<i64>,
) -> HashMap<i64, Point<f64>> {
    match element {
        Element::Node(node) => Some((node.id(), (node.lat(), node.lon()))),
        Element::DenseNode(node) => Some((node.id(), (node.lat(), node.lon()))),
        _ => None,
    }
    .filter(|(node_id, _)| node_ids.contains(node_id))
    .map(|(node_id, lat_lon)| {
        let mut hashmap = HashMap::<i64, Point<f64>>::with_capacity(1);
        let point = Point::from(lat_lon);
        hashmap.insert(node_id, point);
        hashmap
    })
    .unwrap_or_default()
}

pub trait ParMapCollect<Item> {
    fn par_map_collect<Collection>(
        self,
        collector: impl Fn(Element<'_>) -> Collection + Sync + Send,
    ) -> Collection
    where
        Collection: IntoIterator<Item = Item> + Extend<Item> + Default + Sync + Send;
}

impl<Item, R> ParMapCollect<Item> for ElementReader<R>
where
    R: Read + Send,
{
    fn par_map_collect<Collection>(
        self,
        collector: impl Fn(Element<'_>) -> Collection + Sync + Send,
    ) -> Collection
    where
        Collection: IntoIterator<Item = Item> + Extend<Item> + Default + Send + Sync,
    {
        self.par_map_reduce(
            collector,
            || Collection::default(),
            |mut accu, curr| {
                accu.extend(curr);
                accu
            },
        )
        .unwrap()
    }
}

fn get_cyclable_node_ids_from_element(element: Element<'_>) -> HashSet<i64> {
    match element {
        Element::Way(way) => Some(way),
        _ => None,
    }
    .filter(|way| contains_cycleable_tags(way.tags()))
    .map(|way| way.refs().collect::<HashSet<_>>())
    .unwrap_or_default()
}

fn contains_cycleable_tags(tags: TagIter<'_>) -> bool {
    let mut highway_footway = false;
    let mut bicycle_yes = false;

    for tag in tags {
        match tag {
            ("highway", "footway") => {
                highway_footway = true;
            }
            ("bicycle", "yes") => {
                bicycle_yes = true;
            }
            _ => {}
        }

        if highway_footway && bicycle_yes {
            return true;
        }

        if cyclable_way(tag) {
            return true;
        }
    }

    false
}

fn cyclable_way(pair: (&str, &str)) -> bool {
    matches!(
        pair,
        (
            "highway",
            "trunk"
                | "primary"
                | "secondary"
                | "tertiary"
                | "residential"
                | "living_street"
                | "service"
                | "pedestrian"
                | "road"
                | "cycleway"
        ) | ("cycleway", _)
    )
}
