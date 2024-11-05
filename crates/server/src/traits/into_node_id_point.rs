use geo::Point;
use osmpbf::Element;

pub trait IntoNodeIdPoint {
    fn node_id_point(self) -> Option<(i64, Point<f64>)>;
}

impl IntoNodeIdPoint for Element<'_> {
    fn node_id_point(self) -> Option<(i64, Point)> {
        match self {
            Element::Node(node) => Some((node.id(), Point::from((node.lat(), node.lon())))),
            Element::DenseNode(node) => Some((node.id(), Point::from((node.lat(), node.lon())))),
            _ => None,
        }
    }
}
