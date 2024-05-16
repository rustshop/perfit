use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use crate::asset_cache::AssetCache;
use crate::db::{
    AccessTokenRecord, AccessTokenType, AccountRecord, Database, TABLE_ACCESS_TOKENS,
    TABLE_ACCOUNTS, TABLE_ROOT_ACCOUNT,
};
use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::models::{AccountId, MetricId};

#[derive(Debug)]
pub struct AppState {
    pub db: Database,
    pub assets: AssetCache,
}

impl AppState {
    pub async fn init_root_account(
        &self,
        creds_file: &Path,
    ) -> color_eyre::Result<Option<(AccountId, AccessToken)>> {
        self.db
            .write_with(|tx| {
                if tx.open_table(&TABLE_ROOT_ACCOUNT)?.first()?.is_some() {
                    return Ok(None);
                }

                let account_id = AccountId::generate();
                let access_token = AccessToken::generate();

                tx.open_table(&TABLE_ROOT_ACCOUNT)?
                    .insert(&account_id, &())?;

                tx.open_table(&TABLE_ACCOUNTS)?
                    .insert(&account_id, &AccountRecord { created: Ts::now() })?;

                tx.open_table(&TABLE_ACCESS_TOKENS)?.insert(
                    &access_token,
                    &AccessTokenRecord {
                        created: Ts::now(),
                        r#type: AccessTokenType::Root,
                        account_id,
                    },
                )?;

                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(creds_file)?;

                serde_json::to_writer_pretty(
                    &mut file,
                    &json! {
                        {
                            "account-id": account_id,
                            "access-token": access_token,
                        }
                    },
                )?;

                Ok(Some((account_id, access_token)))
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
