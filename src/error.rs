use hyper::http::Error;
use hyper::body::Bytes;

use serde::Deserialize;

use std::{error::Error as StdError, fmt};

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String
}

// implement stderror traits

#[derive(Debug)]
pub enum OsuApiError {
    HyperError { inner: hyper::Error },
    HyperHttpError { inner: Error },
    ApiError { inner: ApiErrorResponse },
    ParsingError { inner: serde_json::Error, body: Bytes },
    BadRequest,
    ServiceUnavailable,
    RateLimited,
    NoToken
}

impl From<Error> for OsuApiError {
    fn from(value: Error) -> Self {
        Self::HyperHttpError {
            inner: value
        }
    }
}

impl From<hyper::Error> for OsuApiError {
    fn from(value: hyper::Error) -> Self {
        Self::HyperError {
            inner: value
        }
    }
}
impl StdError for OsuApiError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            OsuApiError::HyperError { inner } => Some(inner),
            OsuApiError::HyperHttpError { inner } => Some(inner),
            OsuApiError::ApiError { .. } => None,
            OsuApiError::ParsingError { inner, .. } => Some(inner),
            OsuApiError::BadRequest => None,
            OsuApiError::ServiceUnavailable => None,
            OsuApiError::RateLimited => None,
            OsuApiError::NoToken => None,
        }
    }
}


impl fmt::Display for OsuApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsuApiError::HyperError { .. } => f.write_str("hyper error!"),
            OsuApiError::HyperHttpError { .. } => f.write_str("hyper inner error!"),
            OsuApiError::ApiError { .. } => f.write_str("got api error!"),
            OsuApiError::ParsingError { .. } => f.write_str("parsing error!"),
            OsuApiError::BadRequest => f.write_str("bad request!"),
            OsuApiError::ServiceUnavailable => f.write_str("service is unavailable!"),
            OsuApiError::RateLimited => f.write_str("got 429"),
            OsuApiError::NoToken => f.write_str("no token provided!"),
        }
    }
}
