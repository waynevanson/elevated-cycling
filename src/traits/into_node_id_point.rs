use geo::Coord;
use osmpbf::Element;

pub trait IntoNodeIdPoint {
    fn node_id_point(self) -> Option<(i64, Coord<f64>)>;
}

impl IntoNodeIdPoint for Element<'_> {
    fn node_id_point(self) -> Option<(i64, Coord<f64>)> {
        match self {
            Element::Node(node) => Some((node.id(), Coord::from((node.lat(), node.lon())))),
            Element::DenseNode(node) => Some((node.id(), Coord::from((node.lat(), node.lon())))),
            _ => None,
        }
    }
}
