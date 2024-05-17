use std::io;
use std::net::SocketAddr;

use color_eyre::Result;
use futures::Future;
use perfitd::models::access_token::AccessToken;
use perfitd::{opts, Server};
use tempfile::TempDir;
use tracing_subscriber::EnvFilter;

pub fn init_logging() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    Ok(())
}

pub struct PerfitdFixture {
    server: Server,
    #[allow(dead_code)]
    test_dir: TempDir,
    root_access_token: AccessToken,
}

impl PerfitdFixture {
    pub async fn new() -> Result<Self> {
        let test_dir = tempfile::tempdir()?;

        let root_access_token = AccessToken::generate();

        let opts = opts::Opts {
            listen: "[::1]:0".into(),
            db: test_dir.path().join("db.redb"),
            root_access_token: Some(root_access_token),
            ..Default::default()
        };

        let server = perfitd::Server::init(opts).await?;

        Ok(Self {
            server,
            root_access_token,
            test_dir,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        self.server.addr()
    }

    pub fn root_access_token_str(&self) -> String {
        self.root_access_token.to_string()
    }

    pub async fn run(self, test: impl Future<Output = Result<()>>) -> Result<()> {
        tokio::select! {
            res = test => {
                res
            }
            _ = self.server.run() => {
                Err(color_eyre::eyre::format_err!("server failed?"))
            },
        }
    }
}
