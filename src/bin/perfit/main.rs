use std::process::{exit, ExitStatus};
use std::time::Duration;

use clap::Parser as _;
use color_eyre::eyre::bail;
use color_eyre::Result;
use opts::{MetricArgs, ServerArgs};
use perfitd::models::access_token::AccessToken;
use reqwest::header::AUTHORIZATION;
use reqwest::Method;
use serde_json::json;

use crate::opts::Opts;

mod opts;

#[tokio::main]
async fn main() -> Result<()> {
    let _opts = Opts::parse();

    match _opts.cmd {
        opts::Command::Run {
            server_args,
            data_point_args,
            cmd,
            send_on_failure,
            ignore_send_failure,
            metric_args,
        } => {
            let (duration, exit_status) = run_and_time(cmd)?;
            let exit_code = exit_status.code().unwrap_or(255);

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
                if !ignore_send_failure {
                    return Err(err);
                }
                eprintln!("{}", err);
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
        opts::Command::Token(opts::TokenCommand::New { server_args }) => {
            token_new(&server_args).await?;
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

async fn token_new(server_args: &ServerArgs) -> Result<()> {
    let response = make_request(server_args, Method::PUT, "t/", "").await?;
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
    let client = reqwest::Client::new();
    let response = client
        .post(
            server_args
                .server
                .join(&format!("m/{}", metric_args.metric))?,
        )
        .header(
            AUTHORIZATION,
            format!("Bearer {}", server_args.access_token),
        )
        .json(&json! ({
            "value": value,
            "metadata": data_point_args.metadata,
        }))
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        bail!(
            "Http request failed: {status}: {}",
            status.canonical_reason().unwrap_or("unknown response code")
        )
    }
    Ok(())
}

fn run_and_time(cmd: Vec<std::ffi::OsString>) -> Result<(Duration, ExitStatus)> {
    if cmd.is_empty() {
        bail!("Empty command");
    }
    let start = std::time::Instant::now();

    let mut command = std::process::Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    let exit_status = command.spawn()?.wait()?;

    Ok((start.elapsed(), exit_status))
}
