use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

use bincode::{Decode, Encode};
use color_eyre::eyre::bail;
use color_eyre::Result;
use redb_bincode::{ReadTransaction, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::models::access_token::AccessToken;
use crate::models::ts::Ts;
use crate::models::{AccessTokenType, AccountId, MetricId, MetricInternalId};
use crate::routes::error::UserRequestError;

pub const TABLE_DB_VER: TableDefinition<'_, (), u64> = TableDefinition::new("db-ver");

pub const TABLE_ACCOUNTS: TableDefinition<'_, AccountId, AccountRecord> =
    TableDefinition::new("accounts");

pub const TABLE_ACCESS_TOKENS: TableDefinition<'_, AccessToken, AccessTokenRecord> =
    TableDefinition::new("access_tokens");

pub const TABLE_ACCESS_TOKENS_REV: TableDefinition<'_, (AccountId, AccessToken), ()> =
    TableDefinition::new("access_tokens_rev");

pub const TABLE_METRICS: TableDefinition<'_, MetricId, MetricRecord> =
    TableDefinition::new("metrics");

pub const TABLE_METRICS_REV: TableDefinition<'_, MetricInternalId, MetricId> =
    TableDefinition::new("metrics_rev");

pub const TABLE_DATA_POINTS: TableDefinition<'_, DataPoint, DataPointRecord> =
    TableDefinition::new("data_points");

#[derive(Encode, Decode, Clone, Copy)]
pub struct DataPoint {
    pub metric_internal_id: MetricInternalId,
    pub ts: Ts,
    pub idx: u64,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct AccountRecord {
    pub created: Ts,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct AccessTokenRecord {
    pub created: Ts,
    pub account_id: AccountId,
    pub r#type: AccessTokenType,
}

pub const ROOT_ACCOUNT_ID: AccountId = AccountId::from_const(Uuid::from_u128(0));

impl AccessTokenRecord {
    pub fn ensure_can_create_tokens(
        &self,
        auth_account_id: AccountId,
        token_type: AccessTokenType,
    ) -> Result<()> {
        if token_type == AccessTokenType::Root {
            return Err(UserRequestError::Unauthorized.into());
        }

        if auth_account_id == ROOT_ACCOUNT_ID {
            return Err(UserRequestError::RootAccountCantBeUsed.into());
        }

        if matches!(self.r#type, AccessTokenType::Admin) {
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
pub struct MetricRecord {
    pub created: Ts,
    pub account_id: AccountId,
    pub internal_id: MetricInternalId,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct DataPointValue(f32);

impl DataPointValue {
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

/// Metadata attached to a [`DataPoint`]
#[derive(Encode, Decode, Serialize, Debug, Clone, Default)]
pub struct DataPointMetadata(String);

impl DataPointMetadata {
    pub const MAX_LEN: usize = 256;

    pub fn try_new<'a>(s: impl Into<Cow<'a, str>>) -> Result<Self> {
        let s = s.into();
        if Self::MAX_LEN < s.len() {
            bail!("Metadata too long");
        }
        Ok(Self(s.into_owned()))
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for DataPointMetadata {
    type Err = color_eyre::eyre::Report;

    fn from_str(s: &str) -> Result<Self> {
        Self::try_new(s)
    }
}

impl<'de> Deserialize<'de> for DataPointMetadata {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if Self::MAX_LEN < s.len() {
            return Err(serde::de::Error::custom("Metadata too long"));
        }
        Ok(Self(s))
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone)]
pub struct DataPointRecord {
    pub value: DataPointValue,
    pub metadata: DataPointMetadata,
}

#[derive(Debug)]
pub struct Database(redb_bincode::Database);

impl Database {
    const DB_VER: u64 = 0;

    pub async fn init(self) -> Result<Self> {
        self.write_with(|dbtx| {
            dbtx.open_table(&TABLE_ACCOUNTS)?;
            dbtx.open_table(&TABLE_ACCESS_TOKENS)?;
            dbtx.open_table(&TABLE_ACCESS_TOKENS_REV)?;
            dbtx.open_table(&TABLE_METRICS)?;
            dbtx.open_table(&TABLE_METRICS_REV)?;
            dbtx.open_table(&TABLE_DATA_POINTS)?;

            Self::handle_db_ver_migrations(dbtx)?;

            Ok(())
        })
        .await?;

        Ok(self)
    }
    fn handle_db_ver_migrations(dbtx: &WriteTransaction) -> Result<(), color_eyre::eyre::Error> {
        let mut table_db_ver = dbtx.open_table(&TABLE_DB_VER)?;

        let Some(cur_db_ver) = table_db_ver.first()?.map(|g| g.1.value()) else {
            info!("Initializing empty database");
            table_db_ver.insert(&(), &Self::DB_VER)?;

            return Ok(());
        };

        debug!(db_ver = cur_db_ver, "Checking db version");
        if Self::DB_VER < cur_db_ver {
            bail!(
                "Db version {cur_db_ver} higher than code version {}",
                Self::DB_VER
            );
        }

        // migration code will go here

        Ok(())
    }
}

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

    #[instrument(skip_all)]
    pub async fn open(path: impl Into<PathBuf>) -> Result<Database> {
        let path = path.into();
        let create =
            tokio::task::spawn_blocking(move || redb_bincode::Database::create(path)).await??;
        Self::from(create).init().await
    }
}
