use std::io;
use bytes::Bytes;
use tokio_service::Service;
use futures::{future, Future};
use model::request::SmtpCommand;
use model::response::SmtpReply;

pub struct SmtpService;

impl Service for SmtpService {
    // These types must match the corresponding protocol types:
    type Request = SmtpCommand;
    type Response = SmtpReply;

    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;

    // The future for computing the response; box it for simplicity.
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    // Produce a future for computing a response from a request.
    fn call(&self, command: Self::Request) -> Self::Future {

        let reply = match command {
            SmtpCommand::Connect { .. } => SmtpReply::ServiceReadyInfo("me".to_string()),
            _ => SmtpReply::ServiceNotAvailableError(format!("Echo {:?}", command)),
        };

        // In this case, the response is immediate.
        Box::new(future::ok(reply))
    }
}
