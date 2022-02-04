pub use structopt::StructOpt;
use strum::VariantNames;

mod cmd;
mod err;
mod log;
mod main;
pub mod server;

pub type AppResult = Result<(), AppExit>;
pub struct AppExit(i32);

/// Samotop tool and sample server
#[derive(StructOpt, Debug, Clone)]
#[structopt()]
pub struct Main {
    /// Command to run
    #[structopt(subcommand)]
    cmd: Cmd,
    #[structopt(flatten)]
    logging: Logging,
}

/// Samotop tool and sample server
#[derive(StructOpt, Debug, Clone)]
#[structopt()]
pub struct Logging {
    /// configure logging levels (see tracing crate)
    #[structopt(
        global = true,
        long,
        default_value = "warn",
        env = "RUST_LOG",
        name = "log-what"
    )]
    log: String,
    /// how to format the log?
    #[structopt(global=true, long, default_value = "default", env = "RUST_LOG_FORMAT", name="log-fmt", possible_values = LoggingFmt::VARIANTS)]
    log_format: LoggingFmt,
}

#[derive(
    Debug, PartialEq, Copy, Clone, strum_macros::EnumString, strum_macros::EnumVariantNames,
)]
#[strum(serialize_all = "kebab-case")]
pub enum LoggingFmt {
    Default,
    Compact,
    Pretty,
    Json,
}

/// Samotop tool and sample server
#[derive(StructOpt, Debug, Clone)]
#[structopt()]
pub enum Cmd {
    Account,
    Server,
}
