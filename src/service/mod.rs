use std::io;
use bytes::Bytes;
use tokio_service::Service;
use tokio_proto::streaming::{Message, Body};
use futures::{future, Future};
use model::request::SmtpCommand;
use model::response::SmtpReply;

pub struct SmtpService;

impl Service for SmtpService {
    // For non-streaming protocols, service errors are always io::Error
    type Error = io::Error;
    // These types must match the corresponding protocol types:
    type Request = Message<SmtpCommand, Body<Bytes, Self::Error>>;
    type Response = Message<SmtpReply, Body<(), Self::Error>>;

    // The future for computing the response; box it for simplicity.
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    // Produce a future for computing a response from a request.
    fn call(&self, command: Self::Request) -> Self::Future {

        println!("Received {:?}", command);

        let reply = match command {
            Message::WithoutBody(SmtpCommand::Connect { .. }) => {
                Message::WithoutBody(SmtpReply::ServiceReadyInfo("me".to_string()))
            }
            _ => {
                Message::WithoutBody(SmtpReply::ServiceNotAvailableError(
                    format!("Echo {:?}", command),
                ))
            }
        };

        // In this case, the response is immediate.
        Box::new(future::ok(reply))
    }
}
