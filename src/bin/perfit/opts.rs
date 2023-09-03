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
pub struct SampleArgs {
    #[arg(long, env = "PERFIT_SERIES")]
    pub series: String,
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
        sample_args: SampleArgs,

        /// Send the sample even if the `cmd` failed
        #[arg(long)]
        send_on_failure: bool,

        /// Do not fail if unable to send the sample
        #[arg(long)]
        ignore_send_failure: bool,

        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        cmd: Vec<ffi::OsString>,
    },

    /// Report a sample
    Post {
        #[command(flatten)]
        server_args: ServerArgs,

        #[command(flatten)]
        sample_args: SampleArgs,

        sample: f32,
    },

    #[command(subcommand)]
    Account(AccountCommand),

    #[command(subcommand)]
    Series(SeriesCommand),
}

#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    New {
        #[command(flatten)]
        server_args: ServerArgs,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub enum SeriesCommand {
    New {
        #[command(flatten)]
        server_args: ServerArgs,
    },
}
