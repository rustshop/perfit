use std::ffi::OsString;
use std::path::PathBuf;

use axum::http::HeaderValue;
use clap::Parser;
use color_eyre::Result;
use tracing::instrument;

use crate::models::access_token::AccessToken;

fn default_perfit_assets_dir() -> OsString {
    PathBuf::from(env!("PERFITD_SHARE_DIR"))
        .join("assets")
        .into_os_string()
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// Listen address
    #[arg(long, short, default_value = "[::1]:5050", env = "PERFITD_LISTEN")]
    pub listen: String,

    /// Database file path
    #[arg(long, default_value = "perfitd.redb", env = "PERFITD_DB_PATH")]
    pub db: PathBuf,

    /// Cors origin settings
    #[arg(long, env = "PERFITD_CORS_ORIGIN")]
    pub cors_origin: Option<String>,

    /// Root directory of the assets dir
    #[arg(long, env = "PERFITD_ASSETS_DIR", default_value = &default_perfit_assets_dir())]
    pub assets_dir: PathBuf,

    /// If set it will set root account access token to this value
    ///
    /// Can be generated with `perfit token gen`
    #[arg(long, env = "PERFITD_ROOT_ACCESS_TOKEN")]
    pub root_access_token: Option<AccessToken>,

    /// Rate limit replenish token every N microseconds
    #[arg(
        long,
        default_value = "500000",
        env = "PERFITD_RATE_LIMIT_REPLENISH_MICROS"
    )]
    pub rate_limit_replenish_micros: u64,

    /// Rate limit burst size
    #[arg(long, default_value = "60", env = "PERFITD_RATE_LIMIT_BURST")]
    pub rate_limit_burst: u32,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            listen: "[::1]:3000".into(),
            db: "db.redb".into(),
            cors_origin: None,
            assets_dir: default_perfit_assets_dir().into(),
            root_access_token: None,
            rate_limit_replenish_micros: 500000,
            rate_limit_burst: 60,
        }
    }
}

impl Opts {
    #[instrument]
    pub fn cors_origin(&self) -> Result<HeaderValue> {
        Ok(self
            .cors_origin
            .clone()
            .unwrap_or_else(|| format!("http://{}", self.listen))
            .parse()?)
    }
}
