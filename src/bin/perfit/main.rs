use std::process::{exit, ExitStatus};
use std::time::Duration;

use clap::Parser as _;
use color_eyre::eyre::bail;
use color_eyre::Result;
use opts::{MetricArgs, ServerArgs};
use perfitd::models::access_token::AccessToken;
use perfitd::models::AccessTokenType;
use reqwest::header::AUTHORIZATION;
use reqwest::Method;
use serde::Serialize;
use serde_json::json;
use tracing::info;

use crate::opts::Opts;

mod opts;

const LOG_PERFIT: &str = "perfit";

#[tokio::main]
async fn main() -> Result<()> {
    install_tracing();
    let _opts = Opts::parse();

    match _opts.cmd {
        opts::Command::Run {
            server_args,
            data_point_args,
            cmd,
            send_on_failure,
            fail_on_send_failure,
            metric_args,
        } => {
            let (duration, exit_status) = run_and_time(cmd)?;
            let exit_code = exit_status.code().unwrap_or(255);

            info!(target: LOG_PERFIT, duration_millis = duration.as_millis(), exit_code, "Command complete");
            if !exit_status.success() && !send_on_failure {
                exit(exit_code);
            }

            if let Err(err) = send_data_point(
                &server_args,
                &metric_args,
                &data_point_args,
                duration.as_micros() as f32 / 1_000_000.,
            )
            .await
            {
                if fail_on_send_failure {
                    return Err(err);
                }
                eprintln!("Failed to report data point: {}", err);
            }
            exit(exit_code);
        }
        opts::Command::Post {
            server_args,
            data_point_args,
            data_point,
            metric_args,
        } => send_data_point(&server_args, &metric_args, &data_point_args, data_point).await?,

        opts::Command::Account(opts::AccountCommand::New { server_args }) => {
            account_new(&server_args).await?
        }
        opts::Command::Metric(opts::MetricCommand::New { server_args }) => {
            metric_new(&server_args).await?
        }
        opts::Command::Metric(opts::MetricCommand::Get {
            server_args,
            metric_args,
        }) => metric_get(&server_args, &metric_args).await?,
        opts::Command::Token(opts::TokenCommand::Gen) => {
            println!("{}", AccessToken::generate())
        }
        opts::Command::Token(opts::TokenCommand::New {
            server_args,
            r#type,
        }) => {
            token_new(&server_args, &r#type).await?;
        }
    }

    Ok(())
}

async fn make_request(
    server_args: &ServerArgs,
    method: reqwest::Method,
    path: &str,
    body: impl Into<reqwest::Body>,
) -> Result<reqwest::Response> {
    let client = reqwest::Client::new();
    let response = client
        .request(method, server_args.server.join(path)?)
        .header(
            AUTHORIZATION,
            format!("Bearer {}", &server_args.access_token),
        )
        .body(body.into())
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        bail!("Http request failed: {status}",)
    }
    Ok(response)
}

async fn make_request_json<T>(
    server_args: &ServerArgs,
    method: reqwest::Method,
    path: &str,
    payload: &T,
) -> Result<reqwest::Response>
where
    T: Serialize + ?Sized,
{
    let client = reqwest::Client::new();
    let response = client
        .request(method, server_args.server.join(path)?)
        .header(
            AUTHORIZATION,
            format!("Bearer {}", server_args.access_token),
        )
        .json(payload)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        bail!(
            "Http request failed: {status}: {}",
            status.canonical_reason().unwrap_or("unknown response code")
        )
    }
    Ok(response)
}

async fn account_new(server_args: &ServerArgs) -> Result<()> {
    let response = make_request(server_args, Method::PUT, "a/", "").await?;
    println!("{}", response.text().await?);

    Ok(())
}

async fn metric_new(server_args: &ServerArgs) -> Result<()> {
    let response = make_request(server_args, Method::PUT, "m/", "").await?;
    println!("{}", response.text().await?);

    Ok(())
}

async fn token_new(server_args: &ServerArgs, r#type: &AccessTokenType) -> Result<()> {
    let response = make_request_json(
        server_args,
        Method::PUT,
        "t/",
        &json! ({
            "type": r#type,
        }),
    )
    .await?;

    println!("{}", response.text().await?);

    Ok(())
}

async fn metric_get(server_args: &ServerArgs, metric_args: &MetricArgs) -> Result<()> {
    let response = make_request(
        server_args,
        Method::GET,
        &format!("m/{}/json", metric_args.metric),
        "",
    )
    .await?;
    println!("{}", response.text().await?);

    Ok(())
}

async fn send_data_point(
    server_args: &ServerArgs,
    metric_args: &MetricArgs,
    data_point_args: &opts::DataPointArgs,
    value: f32,
) -> Result<()> {
    info!(target: LOG_PERFIT,
         server = %server_args.server,
         metric = %metric_args.metric,
         %value,
         metadata = %data_point_args.metadata.as_deref().unwrap_or(""),
         "Sending data point");
    make_request_json(
        server_args,
        Method::POST,
        &format!("m/{}", metric_args.metric),
        &json! ({
            "value": value,
            "metadata": data_point_args.metadata,
        }),
    )
    .await?;
    Ok(())
}

fn run_and_time(cmd: Vec<std::ffi::OsString>) -> Result<(Duration, ExitStatus)> {
    if cmd.is_empty() {
        bail!("Empty command");
    }

    // Note: showing only num_args, in case there was something sensitive there
    info!(target: LOG_PERFIT, cmd = %&cmd[0].to_string_lossy(), num_args = %cmd.len() - 1, "Running command");
    let start = std::time::Instant::now();

    let mut command = std::process::Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    let exit_status = command.spawn()?.wait()?;

    Ok((start.elapsed(), exit_status))
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_writer(std::io::stderr).with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}
