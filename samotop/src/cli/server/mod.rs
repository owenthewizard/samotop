use super::LoggingFmt;
use structopt::StructOpt;
use strum::VariantNames;

/// Samotop tool and sample server
#[derive(StructOpt, Debug, Clone)]
#[structopt()]
pub struct Config {
    /// How to dispatch mail
    #[structopt(short, long, name = "destination")]
    dispatch: Vec<String>,

    /// how to format the log?
    #[structopt(global=true, long, default_value = "default", env = "RUST_LOG_FORMAT", name="log-fmt", possible_values = LoggingFmt::VARIANTS)]
    log_format: LoggingFmt,
}
