use geo::Point;
use itertools::Itertools;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ElevationResponseElement {
    pub latitude: f32,
    pub longitude: f32,
    pub elevation: f32,
}

#[derive(Debug, Deserialize)]
pub struct ElevationResponse {
    locations: Vec<ElevationResponseElement>,
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
pub async fn lookup_elevation(
    client: Client,
    locations: &Vec<&Point>,
) -> Vec<ElevationResponseElement> {
    let params = from_locations_into_string(locations);
    let url = format!("http://open-elevation:8080/api/v1/lookup?locations={params}");
    println!("{url}");
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