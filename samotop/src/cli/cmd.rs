use super::*;
use tracing::instrument;

impl Cmd {
    /// Execute the main samotop entry point without global/default setup
    #[instrument(level = "trace")]
    pub async fn run(self) -> AppResult {
        println!("{:?}", self);
        match self {
            Cmd::Account => "fail".parse::<u8>()?,
            Cmd::Server => 0,
        };
        Ok(())
    }
}
