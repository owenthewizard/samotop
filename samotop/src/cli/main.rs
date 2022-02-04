use super::*;
use tracing::instrument;

impl Main {
    /// Execute the main samotop entry point
    #[instrument(level = "trace")]
    pub async fn run(self) -> AppResult {
        self.logging.setup()?;
        self.cmd.run().await
    }
}
