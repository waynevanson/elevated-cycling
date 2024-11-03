// app
// setup: read osm, get ways, get nodes, filter cyclable, get nodes, get lat/lon, save

use geo::Point;
use osmpbf::{Element, ElementReader, TagIter};
use redis::{Client, Commands};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
};

fn main() {
    let create_elements = || ElementReader::from_path("").unwrap();
    let client = Client::open("redis://0.0.0.0:6358").unwrap();
    let mut connection = client.get_connection().unwrap();

    let cyclable_node_ids = create_elements().par_map_collect(get_cyclable_node_ids_from_element);

    let points =
        create_elements().par_map_collect(|element| get_nodes_from(element, &cyclable_node_ids));

    let members = points
        .iter()
        .map(|(node_id, point)| (point.x(), point.y(), node_id))
        .collect::<Vec<_>>();

    connection.geo_add::<_, _, ()>("osm", members).unwrap();

    // add redis dockerfile to start before this
    // add this to docker image.
}

fn get_nodes_from(element: Element<'_>, node_ids: &HashSet<i64>) -> HashMap<i64, Point<f64>> {
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
