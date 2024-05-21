mod common;

use color_eyre::Result;
use insta_cmd::get_cargo_bin;
use serde::Deserialize;
use tracing::info;

use crate::common::PerfitdFixture;

trait ExpressionExt {
    fn read_json<T>(self) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    fn read_json_value(self) -> Result<serde_json::Value>
    where
        Self: Sized,
    {
        self.read_json()
    }
}

impl ExpressionExt for duct::Expression {
    fn read_json<T>(self) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let s = self.stdout_capture().read()?;
        let v: T = serde_json::from_str(&s)?;
        Ok(v)
    }
}

#[derive(Deserialize)]
struct NewAccountOutput {
    #[allow(unused)]
    account_id: String,
    access_token: String,
}

#[tokio::test(flavor = "multi_thread")]
async fn sanity_post_data_points() -> Result<()> {
    common::init_logging()?;

    let fixture = PerfitdFixture::new().await?;

    let addr = fixture.addr()?;

    let root_access_token = fixture.root_access_token_str();

    fixture
        .run(async {
            info!("Staring test");
            let bin = get_cargo_bin("perfit");
            tokio::task::spawn_blocking(move || -> Result<_> {
                let NewAccountOutput {
                    account_id: _,
                    access_token,
                } = duct::cmd!(&bin, "account", "new")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", root_access_token)
                    .read_json()?;

                let metric_id: String = duct::cmd!(&bin, "metric", "new")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .read_json()?;

                insta::assert_yaml_snapshot!("no data points", duct::cmd!(&bin, "metric", "get")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .read_json_value()?, {
                    "[].t" => "[ts]",
                });
                duct::cmd!(&bin, "post", "11")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .run()?;

                insta::assert_yaml_snapshot!("one data point", duct::cmd!(&bin, "metric", "get")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .read_json_value()?, {
                    "[].t" => "[ts]",
                });

                duct::cmd!(&bin, "run", "sleep", ".01")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .run()?;

                duct::cmd!(&bin, "post", "12")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .run()?;

                insta::assert_yaml_snapshot!("three data points", duct::cmd!(&bin, "metric", "get")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", &access_token)
                    .env("PERFIT_METRIC", &metric_id)
                    .read_json_value()?, {
                        "[].t" => "[ts]",
                        "[1].v" => "[value]",
                    }
                );

                Ok(())
            })
            .await??;
            Ok(())
        })
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn sanity_user_mgmt() -> Result<()> {
    common::init_logging()?;

    let fixture = PerfitdFixture::new().await?;

    let addr = fixture.addr()?;

    let root_access_token = fixture.root_access_token_str();

    fixture
        .run(async {
            info!("Staring test");
            let bin = get_cargo_bin("perfit");
            tokio::task::spawn_blocking(move || -> Result<_> {
                let NewAccountOutput {
                    account_id: _,
                    access_token,
                } = duct::cmd!(&bin, "account", "new")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", root_access_token)
                    .read_json()?;

                let NewAccountOutput {
                    account_id: _,
                    access_token: _,
                } = duct::cmd!(&bin, "token", "new")
                    .env("PERFIT_SERVER", format!("http://{}", addr))
                    .env("PERFIT_ACCESS_TOKEN", access_token)
                    .read_json()?;

                Ok(())
            })
            .await??;
            Ok(())
        })
        .await
}
