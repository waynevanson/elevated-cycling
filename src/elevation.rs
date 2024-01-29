use derive_more::From;
use geo::Point;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, From)]
pub struct ElevationLocation {
    pub latitude: f64,
    pub longitude: f64,
}

impl From<Point> for ElevationLocation {
    fn from(value: Point) -> Self {
        Self {
            latitude: value.0.x,
            longitude: value.0.y,
        }
    }
}

#[derive(Debug, Serialize, From)]
pub struct ElevationRequestBody {
    locations: Vec<ElevationLocation>,
}

impl FromIterator<Point> for ElevationRequestBody {
    fn from_iter<T: IntoIterator<Item = Point>>(iter: T) -> Self {
        ElevationRequestBody::from(iter.into_iter().map(ElevationLocation::from).collect_vec())
    }
}

#[derive(Debug, Deserialize)]
pub struct LocationAndElevationSuccess {
    pub elevation: f64,
}

#[derive(Debug, Deserialize)]
pub struct LocationAndElevationError {
    pub latitude: f64,
    pub longitude: f64,
    pub error: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LocationAndElevation {
    Success(LocationAndElevationSuccess),
    Error(LocationAndElevationError),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElevationResponse {
    #[serde(rename = "results")]
    Success(Vec<LocationAndElevation>),
    Error(String),
}

// Send request to the API and get the response back.
pub async fn lookup_elevations(client: &Client, body: ElevationRequestBody) -> Vec<f64> {
    let url = "http://open-elevation:8080/api/v1/lookup";

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<ElevationResponse>()
        .await
        .unwrap();

    match response {
        ElevationResponse::Success(success) => success
            .into_iter()
            .map(|location_and_elevation| match location_and_elevation {
                LocationAndElevation::Success(success) => success.elevation,
                error => panic!("{:?}", error),
            })
            .collect_vec(),
        error => panic!("{:?}", error),
    }
}
