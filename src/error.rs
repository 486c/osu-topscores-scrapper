use hyper::http::Error;
use hyper::body::Bytes;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    error: String
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
