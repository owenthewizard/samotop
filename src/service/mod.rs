use std::io;
use bytes::Bytes;
use tokio_service::Service;
use tokio_proto::streaming::{Message, Body};
use futures::{future, Future, Stream};
//use futures::sync::oneshot;
use model::request::SmtpCommand;
use model::response::SmtpReply;

pub struct SmtpService;

type Er = io::Error;
type Req = Message<SmtpCommand, Body<Bytes, Er>>;
type Rsp = Message<SmtpReply, Body<SmtpReply, Er>>;
type Fut = Box<Future<Item = Rsp, Error = Er>>;

impl SmtpService {
    fn write_data(&self, body: Body<Bytes, io::Error>) -> Fut {
        Box::new(
            body
            .inspect(|chunk| info!("data: {:?}", chunk))
            .collect() // convert stream into a future
            .map(|_| Message::WithoutBody(SmtpReply::OkInfo)), //.map_err(|_| Message::WithoutBody(SmtpReply::TransactionFailure))
        )
    }
}

impl Service for SmtpService {
    // For non-streaming protocols, service errors are always io::Error
    type Error = Er;
    // These types must match the corresponding protocol types:
    type Request = Req;
    type Response = Rsp;

    // The future for computing the response; box it for simplicity.
    type Future = Fut;

    // Produce a future for computing a response from a request.
    fn call(&self, command: Req) -> Fut {

        info!("Received {:?}", command);

        match command {
            Message::WithBody(cmd, cmd_body) => {
                match cmd {
                    SmtpCommand::Stream => self.write_data(cmd_body),
                    _ => Box::new(future::ok(Message::WithoutBody(
                        SmtpReply::CommandNotImplementedFailure,
                    ))),
                }
            }
            Message::WithoutBody(cmd) => {
                Box::new(future::ok(Message::WithoutBody(match cmd {
                    SmtpCommand::Connect { peer_addr, .. } => SmtpReply::ServiceReadyInfo(
                        format!("Hi {:?}!", peer_addr),
                    ),
                    SmtpCommand::Data => SmtpReply::StartMailInputChallenge,
                    _ => SmtpReply::CommandNotImplementedFailure,
                })))
            }
        }
    }
}
