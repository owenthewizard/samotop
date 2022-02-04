use super::AppExit;
use tracing::error;

impl<T> From<T> for AppExit
where
    T: std::error::Error,
{
    #[inline]
    fn from(e: T) -> Self {
        error!("Application error - {}", e);
        AppExit(1)
    }
}

impl Into<i32> for AppExit {
    #[inline]
    fn into(self) -> i32 {
        self.0
    }
}
