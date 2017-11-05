pub mod dummy;
pub mod dummyact;

use std::io;
use bytes::Bytes;
use tokio_service::Service;
use tokio_proto::streaming::{Message, Body};
use futures::{future, Future, Stream};
use model::request::{SmtpCommand, SmtpConnection};
use model::response::SmtpReply;

pub struct SmtpService {}

type Er = io::Error;
type Req = Message<SmtpCommand, Body<Bytes, Er>>;
type Rsp = Message<SmtpReply, Body<SmtpReply, Er>>;
type Fut = Box<Future<Item = Rsp, Error = Er>>;

impl SmtpService {
    pub fn new() -> Self {
        Self {}
    }

    fn write_data(&self, body: Body<Bytes, io::Error>) -> Fut {
        // TODO: SmtpCommand::Data => SmtpReply::StartMailInputChallenge,
        // TODO: .map_err(|_| Message::WithoutBody(SmtpReply::TransactionFailure))
        Box::new(
            body
            .inspect(|chunk| info!("data: {:?}", chunk))
            .collect() // convert stream into a future
            .map(|_| Message::WithoutBody(SmtpReply::OkInfo)),
        )
    }
    fn connect(&self, connection: SmtpConnection) -> SmtpReply {
        //self.peer_addr = peer_addr;
        //self.local_addr = local_addr;
        match connection.peer_addr {
            Some(ref a) => SmtpReply::ServiceReadyInfo(format!("Hi {}!", a)),
            _ => SmtpReply::ServiceReadyInfo(format!("Hi there!")),
        }
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
            Message::WithBody(SmtpCommand::Data, cmd_body) => self.write_data(cmd_body),
            Message::WithBody(_, _) => Box::new(future::ok(Message::WithoutBody(
                SmtpReply::CommandNotImplementedFailure,
            ))),
            Message::WithoutBody(cmd) => Box::new(future::ok(Message::WithoutBody(match cmd {
                SmtpCommand::Mail(_mail) => SmtpReply::OkInfo,
                SmtpCommand::Rcpt(_path) => SmtpReply::OkInfo,
                SmtpCommand::Noop(_text) => SmtpReply::OkInfo,
                SmtpCommand::Rset => SmtpReply::OkInfo,
                SmtpCommand::Quit => SmtpReply::ClosingConnectionInfo(format!("Bye!")),
                _ => SmtpReply::CommandNotImplementedFailure,
            }))),

        }
    }
}
