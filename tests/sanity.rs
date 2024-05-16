use std::io;
use std::time::Duration;

use color_eyre::Result;
use insta_cmd::get_cargo_bin;
use perfitd::opts;
use tracing::info;

#[tokio::test(flavor = "multi_thread")]
async fn sanity() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_writer(io::stderr).try_init();

    let test_dir = tempfile::tempdir()?;

    let opts = opts::Opts {
        listen: "[::1]:0".into(),
        db: test_dir.path().join("db.redb"),
        ..Default::default()
    };

    let server = perfitd::Server::init(opts).await?;

    let addr = server.addr()?;

    info!(%addr, "Server port");

    let test = async {
        info!("Staring test");
        let bin = get_cargo_bin("perfit");
        tokio::task::spawn_blocking(move || {
            duct::cmd!(bin, "post", "--metric", "x", "11")
                .env("PERFIT_SERVER", format!("http://{}", addr))
                .run()
        })
        .await??;
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(())
    };

    tokio::select! {
        res = test => {
            res
        }
        _ = server.run() => {
            Err(color_eyre::eyre::format_err!("server failed?"))
        },
    }
}
