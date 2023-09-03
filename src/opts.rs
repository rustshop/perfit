use std::ffi::OsString;
use std::path::PathBuf;

use axum::http::HeaderValue;
use clap::Parser;
use color_eyre::Result;
use tracing::instrument;

fn default_perfit_assets_dir() -> OsString {
    PathBuf::from(env!("PERFIT_SHARE_DIR"))
        .join("assets")
        .into_os_string()
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    #[arg(long, short, default_value = "[::1]:3000")]
    pub listen: String,

    #[arg(long, default_value = "db.redb")]
    pub db: PathBuf,

    #[arg(long, env = "CORS_ORIGIN")]
    cors_origin: Option<String>,

    #[arg(long, env = "PERFIT_ASSETS_DIR", default_value = &default_perfit_assets_dir())]
    pub assets_dir: PathBuf,
}

impl Opts {
    #[instrument]
    pub fn cors_origin(&self) -> Result<HeaderValue> {
        Ok(self
            .cors_origin
            .clone()
            .unwrap_or_else(|| format!("http://{}", self.listen))
            .parse()?)
        //          .ok_or_else(||
        // color_eyre::eyre::anyhow!("failed to parse CORS
        // origin"))
    }
}
