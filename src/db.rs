use std::path::PathBuf;

use bincode::{Decode, Encode};
use color_eyre::Result;
use redb_bincode::{ReadTransaction, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::models::{AccountId, SeriesId, SeriesInternalId};
use crate::routes::error::UserRequestError;

pub const TABLE_ROOT_ACCOUNT: TableDefinition<'_, AccountId, ()> =
    TableDefinition::new("root-account");

pub const TABLE_ACCOUNTS: TableDefinition<'_, AccountId, AccountRecord> =
    TableDefinition::new("accounts");

pub const TABLE_ACCESS_TOKENS: TableDefinition<'_, AccessToken, AccessTokenRecord> =
    TableDefinition::new("access_tokens");

pub const TABLE_SERIES: TableDefinition<'_, SeriesId, SeriesRecord> =
    TableDefinition::new("series");

pub const TABLE_SERIES_REV: TableDefinition<'_, SeriesInternalId, SeriesId> =
    TableDefinition::new("series-rev");

pub const TABLE_SAMPLES: TableDefinition<'_, Sample, SampleRecord> =
    TableDefinition::new("samples");

#[derive(Encode, Decode, Clone, Copy)]
pub struct Sample {
    pub series_internal_id: SeriesInternalId,
    pub ts: Ts,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct AccountRecord {
    pub created: Ts,
}

#[derive(Debug, Encode, Decode, Clone, Copy, Deserialize)]
pub enum AccessTokenType {
    Root,
    Admin,
    Metrics,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct AccessTokenRecord {
    pub created: Ts,
    pub account_id: AccountId,
    pub r#type: AccessTokenType,
}

impl AccessTokenRecord {
    pub fn ensure_can_create_tokens(&self, account_id: AccountId) -> Result<()> {
        if matches!(self.r#type, AccessTokenType::Admin) && account_id == self.account_id {
            return Ok(());
        }

        Err(UserRequestError::Unauthorized.into())
    }
    pub fn ensure_can_create_accounts(self) -> Result<()> {
        if matches!(self.r#type, AccessTokenType::Root) {
            return Ok(());
        }
        Err(UserRequestError::Unauthorized.into())
    }
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct SeriesRecord {
    pub created: Ts,
    pub account_id: AccountId,
    pub internal_id: SeriesInternalId,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct SampleValue(f32);

impl SampleValue {
    pub fn as_f32(self) -> f32 {
        self.0
    }
}
#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct SampleRecord {
    pub value: SampleValue,
}

#[derive(Debug)]
pub struct Database(redb_bincode::Database);

impl From<redb_bincode::Database> for Database {
    fn from(db: redb_bincode::Database) -> Self {
        Self(db)
    }
}

impl Database {
    pub async fn write_with<T>(
        &self,
        f: impl FnOnce(&'_ WriteTransaction) -> Result<T>,
    ) -> Result<T> {
        tokio::task::block_in_place(|| {
            let mut dbtx = self.0.begin_write()?;

            let res = f(&mut dbtx)?;

            dbtx.commit()?;

            Ok(res)
        })
    }

    pub async fn read_with<T>(
        &self,
        f: impl FnOnce(&'_ ReadTransaction) -> Result<T>,
    ) -> Result<T> {
        tokio::task::block_in_place(|| {
            let mut dbtx = self.0.begin_read()?;

            f(&mut dbtx)
        })
    }

    #[instrument]
    pub fn open(path: &PathBuf) -> Result<Database> {
        Ok(Self::from(redb_bincode::Database::create(path)?))
    }
}
