use std::sync::Arc;

use tracing::info;

use crate::asset_cache::AssetCache;
use crate::db::{
    AccessTokenRecord, AccessTokenType, AccountRecord, Database, ROOT_ACCOUNT_ID,
    TABLE_ACCESS_TOKENS, TABLE_ACCESS_TOKENS_REV, TABLE_ACCOUNTS,
};
use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::models::MetricId;

#[derive(Debug)]
pub struct AppState {
    pub db: Database,
    pub assets: AssetCache,
}

impl AppState {
    pub async fn init_root_account(&self, access_token: &AccessToken) -> color_eyre::Result<()> {
        self.db
            .write_with(|tx| {
                tx.open_table(&TABLE_ACCOUNTS)?
                    .insert(&ROOT_ACCOUNT_ID, &AccountRecord { created: Ts::now() })?;

                let access_tokens_table = &mut tx.open_table(&TABLE_ACCESS_TOKENS)?;

                let access_tokens_rev_table = &mut tx.open_table(&TABLE_ACCESS_TOKENS_REV)?;

                if access_tokens_table.get(access_token)?.is_none() {
                    info!("Setting new root account access token");
                    let existing_root_account_access_tokens: Vec<_> = access_tokens_rev_table
                        .range(
                            &(ROOT_ACCOUNT_ID, AccessToken::ZERO)
                                ..=&(ROOT_ACCOUNT_ID, AccessToken::LAST),
                        )?
                        .map(|existing| {
                            let (k, _) = existing?;
                            let (account_id_db, access_token) = k.value();
                            debug_assert_eq!(ROOT_ACCOUNT_ID, account_id_db);

                            Ok(access_token)
                        })
                        .collect::<color_eyre::Result<Vec<_>>>()?;

                    if !existing_root_account_access_tokens.is_empty() {
                        info!(
                            num = existing_root_account_access_tokens.len(),
                            "Deleting previous root account access tokens"
                        );
                    }
                    for existing in existing_root_account_access_tokens {
                        access_tokens_rev_table.remove(&(ROOT_ACCOUNT_ID, existing))?;
                        access_tokens_table.remove(&existing)?;
                    }

                    access_tokens_rev_table.insert(&(ROOT_ACCOUNT_ID, *access_token), &())?;

                    access_tokens_table.insert(
                        access_token,
                        &AccessTokenRecord {
                            created: Ts::now(),
                            r#type: AccessTokenType::Root,
                            account_id: ROOT_ACCOUNT_ID,
                        },
                    )?;
                }

                Ok(())
            })
            .await
    }

    pub fn svg_chart_url(&self, metric_id: MetricId) -> String {
        format!("/s/{}/svg", metric_id)
    }

    pub fn html_chart_url(&self, metric_id: MetricId) -> String {
        format!("/s/{}", metric_id)
    }
}

pub type SharedAppState = Arc<AppState>;
