pub mod mail;
pub mod session;
pub mod tcp;

use model::mail::*;
use tokio::net::TcpStream;
use tokio::prelude::*;

/** 
An object implementing this trait handles TCP connections.

The caller would first `start()` the `Handler`, then pass tcp connections to the handler.

Here's a dead simple implementation that returns the `DeadHandler` as a handler:
```
# extern crate samotop;
# extern crate tokio;
# use samotop;
# use samotop::service::*;
# use tokio::net::TcpStream;
# use tokio::prelude::*;
# use tokio::prelude::future::FutureResult;
#[derive(Clone, Debug)]
pub struct DeadService;

impl TcpService for DeadService {
    type Future = FutureResult<(), ()>;
    fn handle(self, _stream: TcpStream) -> Self::Future {
        future::ok(()) // or do something with the stream
    }
}
```
You can then use this `DeadService` in samotop:
```
# use samotop::service::tcp::DeadService;
let task = samotop::builder()
        .with(DeadService)
        .as_task();
```
*/
pub trait TcpService {
    type Future: Future<Item = (), Error = ()>;
    fn handle(self, stream: TcpStream) -> Self::Future;
}


/** Handles mail sending and has a name */
pub trait MailService {
    type MailDataWrite;
    fn name(&self) -> String;
    fn accept(&self, request: AcceptRecipientRequest) -> AcceptRecipientResult;
    fn mail(&self, envelope: Envelope) -> Option<Self::MailDataWrite>;
}


pub trait MailHandler {
    fn queue(self) -> QueueResult;
}

pub trait SessionService {
    type Handler;
    fn start(&self) -> Self::Handler;
}
