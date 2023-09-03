use std::str::FromStr as _;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use super::{RequestError, UserRequestError};
use crate::db::{self, AccessTokenRecord};
use crate::models::access_token::AccessToken;
use crate::state::SharedAppState;

#[derive(Debug)]
pub struct Auth(pub AccessTokenRecord);

#[async_trait]
impl FromRequestParts<SharedAppState> for Auth {
    type Rejection = RequestError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedAppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .filter(|val| val.starts_with("Bearer "))
            .ok_or(UserRequestError::MissingAuthorizationToken)?;

        let token = auth_header.trim_start_matches("Bearer ");

        let access_token = AccessToken::from_str(token)
            .map_err(|_e| UserRequestError::MalformedAuthoraizationToken)?;

        let record = state
            .db
            .read_with(|tx| {
                Ok(tx
                    .open_table(&db::TABLE_ACCESS_TOKENS)?
                    .get(&access_token)?
                    .ok_or(UserRequestError::InvalidAuthorizationToken)?
                    .value())
            })
            .await?;

        Ok(Auth(record))
    }
}
