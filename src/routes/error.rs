use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use tracing::info;

use super::AppJson;
use crate::models::MetricId;

#[derive(Debug, Error)]
pub enum UserRequestError {
    #[error("Metric Not Found: {0}")]
    MetricNotFound(MetricId),
    #[error("Invalid Path")]
    InvalidPath,
    #[error("Unauthorized - Missing Authorization Token")]
    MissingAuthorizationToken,
    #[error("Unauthorized - Malformed Authorization Token")]
    MalformedAuthoraizationToken,
    #[error("Unauthorized - Invalid Authorization Token")]
    InvalidAuthorizationToken,
    #[error("Bad Request - Can't use root account")]
    RootAccountCantBeUsed,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Format Not Supported")]
    FormatNotSupported,
    #[error("Internal Server Error")]
    AssertionError,
}
/// As we can't impl IntoResponse for external errors, we need to wrap it in
/// a newtype
pub struct RequestError(color_eyre::eyre::Error);

pub type RequestResult<T> = std::result::Result<T, RequestError>;

impl From<color_eyre::eyre::Error> for RequestError {
    fn from(value: color_eyre::eyre::Error) -> Self {
        Self(value)
    }
}
impl From<UserRequestError> for RequestError {
    fn from(value: UserRequestError) -> Self {
        Self(value.into())
    }
}

// How we want errors responses to be serialized
#[derive(Serialize)]
pub struct UserErrorResponse {
    pub message: String,
}

impl IntoResponse for &UserRequestError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            UserRequestError::Unauthorized | UserRequestError::InvalidAuthorizationToken => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            UserRequestError::AssertionError
            | UserRequestError::InvalidPath
            | UserRequestError::RootAccountCantBeUsed
            | UserRequestError::MissingAuthorizationToken
            | UserRequestError::MalformedAuthoraizationToken
            | UserRequestError::MetricNotFound(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            UserRequestError::FormatNotSupported => (StatusCode::NOT_FOUND, self.to_string()),
        };
        (status_code, AppJson(UserErrorResponse { message })).into_response()
    }
}
impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        info!(err=%self.0, "Request Error");

        let (status_code, message) =
            if let Some(user_err) = self.0.root_cause().downcast_ref::<UserRequestError>() {
                return user_err.into_response();
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Service Error".to_owned(),
                )
            };

        (status_code, AppJson(UserErrorResponse { message })).into_response()
    }
}
