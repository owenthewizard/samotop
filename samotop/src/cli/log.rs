use super::*;
use tracing::instrument;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

impl Logging {
    /// Set up application logging
    #[instrument(level = "trace")]
    pub fn setup(self) -> AppResult {
        std::env::set_var("RUST_LOG", &self.log);
        let builder = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(FmtSpan::ACTIVE)
            .with_writer(std::io::stderr);
        match self.log_format {
            LoggingFmt::Default => tracing::subscriber::set_global_default(builder.finish()),
            LoggingFmt::Json => tracing::subscriber::set_global_default(builder.json().finish()),
            LoggingFmt::Compact => {
                tracing::subscriber::set_global_default(builder.compact().finish())
            }
            LoggingFmt::Pretty => {
                tracing::subscriber::set_global_default(builder.pretty().finish())
            }
        }?;
        tracing_log::LogTracer::init()?;
        Ok(())
    }
}
