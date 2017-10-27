use std::io;
use tokio_service::Service;
use futures::{future, Future};
use model::request::{SmtpInput, SmtpCommand};
use model::response::SmtpReply;

pub struct SmtpService {}

type Er = io::Error;
type Req = SmtpInput;
type Rsp = SmtpReply;
type Fut = Box<Future<Item = Rsp, Error = Er>>;

impl SmtpService {
    pub fn new() -> Self {
        Self {}
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
    fn call(&self, inp: Req) -> Fut {

        info!("Received {:?}", inp);

        Box::new(future::ok(match inp {
            SmtpInput::Connect(_c) => SmtpReply::ServiceReadyInfo(format!("Hi!")),
            SmtpInput::Command(_, _, cmd) => {
                match cmd {
                    SmtpCommand::Connect(_c) => SmtpReply::ServiceReadyInfo(format!("Hi!")),
                    SmtpCommand::Helo(_) => SmtpReply::OkInfo,
                    SmtpCommand::Mail(_mail) => SmtpReply::OkInfo,
                    SmtpCommand::Rcpt(_path) => SmtpReply::OkInfo,
                    SmtpCommand::Data => SmtpReply::StartMailInputChallenge,
                    SmtpCommand::EndOfStream => SmtpReply::None,
                    SmtpCommand::Noop(_text) => SmtpReply::OkInfo,
                    SmtpCommand::Rset => SmtpReply::OkInfo,
                    SmtpCommand::Quit => SmtpReply::ClosingConnectionInfo(format!("Bye!")),
                    _ => SmtpReply::CommandNotImplementedFailure,
                }
            }
            _ => SmtpReply::CommandNotImplementedFailure,
        }))
    }
}
