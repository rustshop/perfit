use std::ffi;

use clap::{Args, Parser, Subcommand};
use url::Url;

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Args, Clone, Debug)]
pub struct ServerArgs {
    #[arg(long, env = "PERFIT_SERVER")]
    pub server: Url,

    #[arg(long, env = "PERFIT_AUTH_TOKEN")]
    pub auth_token: String,
}

#[derive(Args, Clone, Debug)]
pub struct DataPointArgs {
    #[arg(long, env = "PERFIT_METRIC")]
    pub metric: String,

    #[arg(long, env = "PERFIT_METADATA")]
    pub metadata: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    /// Report the duration it took to execute a command
    // Make `--help` be passed to cmd, not us
    #[command(disable_help_flag = true)]
    Run {
        #[command(flatten)]
        server_args: ServerArgs,

        #[command(flatten)]
        data_point_args: DataPointArgs,

        /// Send the data point even if the `cmd` failed
        #[arg(long)]
        send_on_failure: bool,

        /// Do not fail if unable to send the data point
        #[arg(long)]
        ignore_send_failure: bool,

        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        cmd: Vec<ffi::OsString>,
    },

    /// Report a data point
    Post {
        #[command(flatten)]
        server_args: ServerArgs,

        #[command(flatten)]
        data_point_args: DataPointArgs,

        data_point: f32,
    },

    #[command(subcommand)]
    Account(AccountCommand),

    #[command(subcommand)]
    Metric(MetricCommand),
}

#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    New {
        #[command(flatten)]
        server_args: ServerArgs,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum MetricCommand {
    New {
        #[command(flatten)]
        server_args: ServerArgs,
    },
}
