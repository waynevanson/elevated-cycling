use crate::partition_results::PartitionResults;
use derive_more::From;
use geo::Point;
use itertools::Itertools;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Debug, Deserialize, Error)]
#[error("{error}")]
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

impl TryFrom<LocationAndElevation> for LocationAndElevationSuccess {
    type Error = LocationAndElevationError;

    fn try_from(value: LocationAndElevation) -> Result<Self, Self::Error> {
        match value {
            LocationAndElevation::Success(ok) => Ok(ok),
            LocationAndElevation::Error(error) => Err(error),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElevationResponse {
    #[serde(rename = "results")]
    Success(Vec<LocationAndElevation>),
    Error(String),
}

impl TryFrom<ElevationResponse> for Vec<LocationAndElevation> {
    type Error = String;

    fn try_from(value: ElevationResponse) -> Result<Self, String> {
        match value {
            ElevationResponse::Success(ok) => Ok(ok),
            ElevationResponse::Error(error) => Err(error),
        }
    }
}

const ELEVATION_ENDPOINT: &str = "http://open-elevation:8080/api/v1/lookup";

#[derive(Debug, Error)]

pub enum ElevationsError {
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("{0:?}")]
    FailedRequest(String),

    #[error("{0:?}")]
    FailedElevation(Vec<LocationAndElevationError>),
}

pub async fn lookup_elevations(
    client: &Client,
    body: ElevationRequestBody,
) -> Result<Vec<f64>, ElevationsError> {
    let response = client
        .post(ELEVATION_ENDPOINT)
        .json(&body)
        .send()
        .await?
        .json::<ElevationResponse>()
        .await?;

    let locations =
        Vec::<LocationAndElevation>::try_from(response).map_err(ElevationsError::FailedRequest)?;

    let elevations = locations
        .into_iter()
        .map(|loc_and_ele| LocationAndElevationSuccess::try_from(loc_and_ele))
        .map(|result| result.map(|success| success.elevation))
        .partition_results::<Vec<_>, Vec<_>>()
        .map_err(ElevationsError::FailedElevation)?;

    Ok(elevations)
}
