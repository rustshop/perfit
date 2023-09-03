use std::process::{exit, ExitStatus};
use std::time::Duration;

use clap::Parser as _;
use color_eyre::eyre::bail;
use color_eyre::Result;
use opts::ServerArgs;
use reqwest::header::AUTHORIZATION;
use reqwest::Method;

use crate::opts::Opts;

mod opts;

#[tokio::main]
async fn main() -> Result<()> {
    let _opts = Opts::parse();

    match _opts.cmd {
        opts::Command::Run {
            server_args,
            sample_args,
            cmd,
            send_on_failure,
            ignore_send_failure,
        } => {
            let (duration, exit_status) = run_and_time(cmd)?;
            let exit_code = exit_status.code().unwrap_or(255);

            if !exit_status.success() && !send_on_failure {
                exit(exit_code);
            }

            if let Err(err) = send_sample(
                &server_args,
                &sample_args,
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
            sample_args,
            sample,
        } => send_sample(&server_args, &sample_args, sample).await?,

        opts::Command::Account(opts::AccountCommand::New { server_args }) => {
            account_new(&server_args).await?
        }
        opts::Command::Series(opts::SeriesCommand::New { server_args }) => {
            series_new(&server_args).await?
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
        .header(AUTHORIZATION, format!("Bearer {}", server_args.auth_token))
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

async fn series_new(server_args: &ServerArgs) -> Result<()> {
    let response = make_request(server_args, Method::PUT, "s/", "").await?;
    println!("{}", response.text().await?);

    Ok(())
}

async fn send_sample(
    server_args: &ServerArgs,
    sample_args: &opts::SampleArgs,
    sample: f32,
) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post(
            server_args
                .server
                .join(&format!("s/{}", sample_args.series))?,
        )
        .header(AUTHORIZATION, format!("Bearer {}", server_args.auth_token))
        .json(&sample)
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
