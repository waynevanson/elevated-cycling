use derive_more::From;
use geo::Point;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, From)]
pub struct ElevationLocation {
    pub latitude: f64,
    pub longitude: f64,
}

impl From<&Point> for ElevationLocation {
    fn from(value: &Point) -> Self {
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

#[derive(Debug, Deserialize)]
pub struct LocationAndElevation {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
}

#[derive(Debug, Deserialize)]
pub struct ElevationResponseSuccess {
    locations: Vec<LocationAndElevation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElevationResponse {
    #[serde(rename = "results")]
    Success(ElevationResponseSuccess),
    Error(String),
}

// Send request to the API and get the response back.
pub async fn lookup_elevation(
    client: Client,
    body: &ElevationRequestBody,
) -> Vec<LocationAndElevation> {
    let url = format!("http://open-elevation:8080/api/v1/lookup");

    println!("{:?}", &body);
    let response = client
        .post(url)
        .json(body)
        .send()
        .await
        .unwrap()
        .json::<ElevationResponse>()
        .await
        .unwrap();

    match response {
        ElevationResponse::Success(success) => success.locations,
        error => panic!("{:?}", error),
    }
}
