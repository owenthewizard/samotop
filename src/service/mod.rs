pub mod mail;
pub mod session;
pub mod tcp;

use model::controll::*;
use model::mail::*;
use tokio::net::TcpStream;
use tokio::prelude::*;

/** 
An object implementing this trait handles TCP connections in a `Future`.

The caller would ask the service to `handle()` the `TcpStream`, 
then poll the returned future or more likely `tokio::spawn()` it.

Here's a dead simple implementation that returns a completed future 
and doesn't do anything with the stream:

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
        .build_task();
```

The `SamotopService` implements this trait.
*/
pub trait TcpService {
    type Future: Future<Item = (), Error = ()>;
    fn handle(self, stream: TcpStream) -> Self::Future;
}

/**
The service which implements this trait has a name.
*/
pub trait NamedService {
    fn name(&self) -> String;
}

/**
A mail guard can be queried whether a recepient is accepted on on which address.
*/
pub trait MailGuard {
    type Future: Future<Item = AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future;
}

/**
A mail queue allows us to queue an e-mail. 
We start with an envelope. Then, depending on implementation, 
the `Mail` implementation receives the e-mail body.
Finally, the caller queues the mail by calling `Mail.queue()`.
*/
pub trait MailQueue {
    type Mail;
    type MailFuture: Future<Item = Option<Self::Mail>>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture;
}

/**
The final step of sending a mail is queueing it for delivery.
*/
pub trait Mail {
    fn queue(self) -> QueueResult;
}

/**
A session service handles the Samotop session
*/
pub trait SessionService {
    type Handler;
    fn start(&self, tls_conf: TlsControll) -> Self::Handler;
}
