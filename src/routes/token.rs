use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;

use super::auth::Auth;
use super::error::RequestResult;
use crate::db::{AccessTokenRecord, AccessTokenType, TABLE_ACCESS_TOKENS, TABLE_ACCESS_TOKENS_REV};
use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::state::SharedAppState;

#[derive(Deserialize, Debug)]
pub struct TokenNewOpts {
    r#type: AccessTokenType,
}

#[instrument]
pub async fn token_new(
    State(state): State<SharedAppState>,
    Auth(auth): Auth,
    Json(payload): Json<TokenNewOpts>,
) -> RequestResult<Json<serde_json::Value>> {
    auth.ensure_can_create_tokens(payload.r#type)?;
    let token = AccessToken::generate();

    let account_id = auth.account_id;

    state
        .db
        .write_with(|tx| {
            tx.open_table(&TABLE_ACCESS_TOKENS)?.insert(
                &token,
                &AccessTokenRecord {
                    created: Ts::now(),
                    r#type: payload.r#type,
                    account_id,
                },
            )?;

            tx.open_table(&TABLE_ACCESS_TOKENS_REV)?
                .insert(&(account_id, token), &())?;

            Ok(())
        })
        .await?;

    Ok(Json(json!({
        "account_id": account_id,
        "access_token": token,
    })))
}
