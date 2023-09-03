use axum::extract::State;
use axum::Json;
use serde_json::json;
use tracing::instrument;

use super::auth::Auth;
use crate::db::{
    AccessTokenRecord, AccessTokenType, AccountRecord, TABLE_ACCESS_TOKENS, TABLE_ACCOUNTS,
};
use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::models::AccountId;
use crate::routes::error::RequestResult;
use crate::state::SharedAppState;

#[instrument]
pub async fn account_new(
    State(state): State<SharedAppState>,
    Auth(auth): Auth,
) -> RequestResult<Json<serde_json::Value>> {
    auth.ensure_can_create_accounts()?;
    let account_id = AccountId::generate();
    let admin_token = AccessToken::generate();

    state
        .db
        .write_with(|tx| {
            tx.open_table(&TABLE_ACCOUNTS)?
                .insert(&account_id, &AccountRecord { created: Ts::now() })?;

            tx.open_table(&TABLE_ACCESS_TOKENS)?.insert(
                &admin_token,
                &AccessTokenRecord {
                    created: Ts::now(),
                    r#type: AccessTokenType::Admin,
                    account_id,
                },
            )?;

            Ok(())
        })
        .await?;

    Ok(Json(json!({
        "account-id": account_id,
        "access-token": admin_token,
    })))
}
