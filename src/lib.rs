mod asset_cache;
mod db;
mod fragment;
pub mod models;
pub mod opts;
mod routes;
mod state;

use std::net::SocketAddr;
use std::str::FromStr as _;
use std::sync::Arc;
use std::time::Duration;

use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::{HeaderName, Method};
use axum::Router;
use color_eyre::Result;
use db::Database;
use tokio::net::{TcpListener, TcpSocket};
use tokio::signal;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::compression::predicate::SizeAbove;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::CompressionLevel;
use tracing::{debug, info};

use crate::asset_cache::AssetCache;
use crate::state::AppState;

pub struct Server {
    opts: opts::Opts,
    listener: TcpListener,
    state: Arc<AppState>,
}

impl Server {
    pub async fn init(opts: opts::Opts) -> Result<Server> {
        let assets = AssetCache::load_files(&opts.assets_dir).await;
        let listener = Self::get_listener(&opts).await?;
        let db = Database::open(&opts.db).await?;
        let state = Arc::new(AppState { db, assets });

        if let Some(access_token) = opts.root_access_token {
            state.init_root_account(&access_token).await?;
        }
        info!("Listening on {}", listener.local_addr()?);
        Ok(Self {
            listener,
            opts,
            state,
        })
    }

    pub async fn get_listener(opts: &opts::Opts) -> Result<TcpListener> {
        let addr = SocketAddr::from_str(&opts.listen)?;

        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };

        if opts.reuseport {
            #[cfg(unix)]
            socket.set_reuseport(true)?;
        }
        socket.set_nodelay(true)?;

        socket.bind(addr)?;
        Ok(socket.listen(1024)?)
    }

    // TODO: move more stuff to init
    pub async fn run(self) -> Result<()> {
        let governor_conf = Box::new(
            GovernorConfigBuilder::default()
                .per_millisecond(self.opts.rate_limit_replenish_millis)
                .burst_size(self.opts.rate_limit_burst)
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
                debug!("Rate limiting storage size: {}", governor_limiter.len());
            }
        });

        let router = Router::new()
            .merge(routes::route_handler(self.state.clone()))
            .nest("/assets", routes::static_file_handler(self.state.clone()));

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
