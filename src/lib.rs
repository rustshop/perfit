mod asset_cache;
mod db;
mod fragment;
mod models;
pub mod opts;
mod routes;
mod serde;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::{HeaderName, Method};
use axum::Router;
use color_eyre::Result;
use db::Database;
use tokio::signal;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::compression::predicate::SizeAbove;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::CompressionLevel;
use tracing::info;

use crate::asset_cache::AssetCache;
use crate::state::AppState;

pub struct Server {
    opts: opts::Opts,
    listener: tokio::net::TcpListener,
}

impl Server {
    pub async fn init(opts: opts::Opts) -> Result<Server> {
        let listener = tokio::net::TcpListener::bind(opts.listen.clone()).await?;

        info!("Listening on {}", listener.local_addr()?);
        Ok(Self { listener, opts })
    }

    // TODO: move more stuff to init
    pub async fn run(self) -> Result<()> {
        let governor_conf = Box::new(
            GovernorConfigBuilder::default()
                .per_second(2)
                .burst_size(20)
                .finish()
                .unwrap(),
        );

        let governor_limiter = governor_conf.limiter().clone();
        let interval = Duration::from_secs(60);
        // a separate background task to clean up
        std::thread::spawn(move || loop {
            std::thread::sleep(interval);
            governor_limiter.retain_recent();
            if 0 < governor_limiter.len() {
                tracing::info!("Rate limiting storage size: {}", governor_limiter.len());
            }
        });

        let db = Database::open(&self.opts.db)?;
        let assets = AssetCache::load_files(&self.opts.assets_dir).await;
        let state = Arc::new(AppState { db, assets });

        state
            .init_root_account(&self.opts.db.with_extension("creds"))
            .await?;

        let router = Router::new()
            .merge(routes::route_handler(state.clone()))
            .nest("/assets", routes::static_file_handler(state.clone()));

        info!("Starting server");
        axum::serve(
            self.listener,
            router
                .layer(cors_layer(&self.opts)?)
                .layer(compression_layer())
                .layer(GovernorLayer {
                    config: Box::leak(governor_conf),
                })
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await?;

        Ok(())
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
}

fn compression_layer() -> CompressionLayer<SizeAbove> {
    CompressionLayer::new()
        .quality(CompressionLevel::Precise(4))
        .compress_when(SizeAbove::new(512))
}

fn cors_layer(opts: &opts::Opts) -> Result<CorsLayer> {
    Ok(CorsLayer::new()
        .allow_credentials(true)
        .allow_headers([ACCEPT, CONTENT_TYPE, HeaderName::from_static("csrf-token")])
        .max_age(Duration::from_secs(86400))
        .allow_origin(opts.cors_origin()?)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
            Method::PATCH,
        ]))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
