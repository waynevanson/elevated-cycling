use geo::Point;
use itertools::Itertools;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Location {
    pub latitude: f32,
    pub longitude: f32,
    pub elevation: f32,
}

#[derive(Debug, Deserialize)]
pub struct ElevationResponse {
    locations: Vec<Location>,
}

fn from_locations_into_string(locations: &Vec<&Point>) -> String {
    locations
        .into_iter()
        .map(|point| point.x_y())
        .map(|(x, y)| format!("{x},{y}"))
        .intersperse('|'.to_string())
        .collect()
}

// Send request to the API and get the response back.
pub async fn lookup_elevation(client: Client, locations: &Vec<&Point>) -> Vec<Location> {
    let params = from_locations_into_string(locations);
    let url = format!("http://open-elevation:8080/api/v1/lookup?locations={params}");
    client
        .get(url)
        .send()
        .await
        .unwrap()
        .json::<ElevationResponse>()
        .await
        .unwrap()
        .locations
}

#[cfg(test)]
mod test {
    use super::*;
    use geo::Point;

    #[test]
    fn transform_locations_into_pipe_separated() {
        let coords = [(32.0, 2.0), (99.9, 322.1)]
            .into_iter()
            .map(Point::from)
            .collect_vec();
        let locations = coords.iter().collect_vec();
        let string = from_locations_into_string(&locations);

        assert_eq!(string, "32,2|99.9,322.1");
    }
}
