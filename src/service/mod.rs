pub mod console;
pub mod dead;
pub mod samotop;

use model::mail::Envelope;

/** 
An object implementing this trait handles TCP connections.

The caller would first `start()` the `Handler`, then pass tcp connections to the handler.

Here's a dead simple implementation that returns the `DeadHandler` as a handler:
```
# extern crate samotop;
# extern crate tokio;
# use samotop;
# use samotop::service::*;
# use tokio::io;
# use tokio::net::TcpStream;
# use tokio::prelude::*;
# 
#[derive(Clone, Debug)]
pub struct DeadService;

impl TcpService for DeadService {
    type Handler = DeadHandler;
    fn start(&self) -> Self::Handler {
        DeadHandler
    }
}
pub struct DeadHandler;
```
While this would satisfy the trait, you'll want some more magic. 
For it to be usable in Samotop, implement `Sink` for the `DeadHandler`. 
The sink accepts `tokio::net::TcpStream` and we work with `io::Error`.
```
# extern crate samotop;
# extern crate tokio;
# use samotop;
# use samotop::service::*;
# use tokio::io;
# use tokio::net::TcpStream;
# use tokio::prelude::*;
# 
# #[derive(Clone, Debug)]
# pub struct DeadService;
# 
# impl TcpService for DeadService {
#     type Handler = DeadHandler;
#     fn start(&self) -> Self::Handler {
#         DeadHandler
#     }
# }
# pub struct DeadHandler;
impl Sink for DeadHandler {
    type SinkItem = TcpStream;
    type SinkError = io::Error;

    fn start_send(&mut self, _item: Self::SinkItem)
            -> io::Result<AsyncSink<Self::SinkItem>> {
        println!("got an item");
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> io::Result<Async<()>> {
        Ok(Async::Ready(()))
    }
}
# fn test () {
#     let task = samotop::builder()
#         .with(DeadService)
#         .as_task();
# }
```
You can then use this `DeadService` in samotop:
```
# use samotop::service::dead::DeadService;
let task = samotop::builder()
        .with(DeadService)
        .as_task();
```
*/
pub trait TcpService {
    /// The handler that receives TCP connections.
    /// Typically a `Sink<SinkItem = tokio::net::TcpStream,
    /// SinkError = io::Error>` implementation.
    type Handler;
    /// Start the `Handler`.
    fn start(&self) -> Self::Handler;
}

/** Handles mail sending and has a name */
pub trait MailService {
    type MailDataWrite;
    fn name(&mut self) -> &str;
    fn send(&mut self, envelope: Envelope) -> Option<Self::MailDataWrite>;
}
