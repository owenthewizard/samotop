use std::io;
use tokio_service::Service;
use futures::{future, Future};
use model::act::{Act, ActResult};

pub struct SmtpService {}

type Er = io::Error;
type Req = Act;
type Rsp = ActResult;
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
    fn call(&self, act: Req) -> Fut {
        info!("Received {:?}", act);

        Box::new(future::ok(match act {
            Act::Mail(_) => Ok(act),
        }))
    }
}
