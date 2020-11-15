use crate::common::*;
use crate::mail::Transaction;
use std::fmt;

pub trait SmtpSession {
    fn transaction(&self) -> &Transaction;
    fn transaction_mut(&mut self) -> &mut Transaction;
    #[must_use = "future must be polled"]
    fn say(&mut self, what: &dyn fmt::Display) -> Pin<Box<dyn Future<Output = Result<()>>>>;
    fn start_tls(&mut self) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}
