/*!
This is an SMTP server library with focus on privacy.
There is also an actual SMTP server - see [samotop-server](https://gitlab.com/BrightOpen/BackYard/Samotop/-/tree/develop/samotop-server)

SMTP Relay Server (MTA) library for Rust
with focus on spam elimination and privacy.
The motivation is to revive e-mail infrastructure
and architecture, address current problems
and allow new systems to integrate SMTP.
It's called SaMoToP, which could be a nice Czech word.

# Status
Reaching stable. The API builds on async/await to offer a convenient asynchronous interface.

# Installation


# Usage
See the docs on [docs.rs](https://docs.rs/samotop).

Add this to your `Cargo.toml`:

```toml
[dependencies.samotop]
version = "0"
```

Note that the API is still unstable. Please use the latest release.

There are a few interesting provisions one could take away from Samotop:
* The server (through `samotop::server::Server`) - it takes IP:port's to listen `on()` and you can then `serve()` your own implementation of a `TcpService`.
* The SMTP service (`SmtpService`) - it takes an async IO and provides an SMTP service defined by `SessionService`.
* The low level `SmtpCodec` - it translates between IO and a `Stram` of `ReadControl` and a `Sink` of `WriteControl`. It handles SMTP mail data as well.
* The SMTP session parser (`SmtpParser`) - it takes `&str` and returns parsed commands or session.
* The SMTP session and domain model (`samotop::model::session`, `samotop::model::smtp`) - these describe the domain and behavior.
* The mail handling stuff that is yet to be written (`MailService`)...

## SMTP Server

You can run a plaintext SMTP service without support for STARTTLS.

```no_run
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::Server;
use samotop::service::tcp::DummyTcpService;
fn main() {
    env_logger::init();
    let mail = samotop::service::mail::default::DefaultMailService;
    let sess = samotop::service::session::StatefulSessionService::new(mail);
    let svc = samotop::service::tcp::SmtpService::new(sess);
    let svc = samotop::service::tcp::TlsEnabled::disabled(svc);
    let srv = samotop::server::Server::on("localhost:25").serve(svc);
    async_std::task::block_on(srv).unwrap()
}
```

To enable TLS, provide a rustls TlsAcceptor.
Alternatively, implement TlsEnabled for another TLS library.
See the samotop-server for fullexample with TLS enabled.

## Dummy server
Any TCP service can be served. See the docs for `TcpService`.
Run it with `RUST_LOG=trace` to display trace log.
Use this to understand how networking IO is handled.

```no_run
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::Server;
use samotop::service::tcp::DummyTcpService;
fn main() {
    env_logger::init();
    let mut srv = Server::on("localhost:0").serve(DummyTcpService);
    async_std::task::block_on(srv).unwrap()
}
```
# Development

Refresh README from docs:
`$ cargo readme > README.md`

# Status
We've got a decent SMTP command parser written as a PEG grammar. The model is tightly nit from the RFCs. An async-std based server will hear your SMTP commands, drive the SMTP state machine and correct you if you step aside. Once a mail session is ready, the mail data are currently dumped to the console. After that, you can do it again. See the [crate documentation](https://docs.rs/samotop/) for API status. The [samotop crate is published on crates.io](https://crates.io/crates/samotop).
The executable is not very useful yet except for debugging SMTP itself until common MDA/MTA features are implemented.

## Done
- [x] Parse SMTP commands and write responses according to RFCs
- [x] SMTP state machine - helo, mail, rcpt*, data, rset, quit - must be in correct order according to RFCs
- [x] DATA are handled and terminated correctly (escape dot, final dot).
- [x] Async/await with async-std backing
- [x] Privacy: TLS/STARTTLS supported using rustls
- [x] MTA: Simple mail relay, logging smtp session to standard output but able to receive mail from common relays
- [x] MDA: System-wide mailbox - mailbox for all unclaimed domains / addresses - store mail in a folder so it can be processed further
- [x] Antispam: SPF (through viaspf, todo:async)

## To do
- [ ] Antispam: Strict SMTP (require CRLF, reject if client sends mail before banner or EHLO response)
- [ ] Antispam: whitelist and blacklist
- [ ] Antispam: greylisting
- [ ] Antispam: white/black/grey list with UI - user decides new contact handling
- [ ] Antispam: is it encrypted?
- [ ] Antispam: reverse lookup
- [ ] Antispam: DANE (DNSSEC) with UI - user verifies signatures
- [ ] Processing: Relay mail to another MTA
- [ ] Processing: Store mail in Maildir (MDA)
- [ ] MDA: Domain mailbox - mailbox for unclaimed addresses
- [ ] MDA: User mailbox - mailbox for specific address or alias
- [ ] MDA: Smart mailbox - multiple mailbox addresses by convention
- [ ] Privacy: Refuse unencrypted session
- [ ] Privacy: Encryption at rests, encrypt e-mails, only the recipient will be able to decrypt
- [ ] Privacy: Leave no trace, no logs, obfuscated file dates...

# Company
In Rust world I have so far found mostly SMTP clients.

## SMTP server
* [mailin](https://crates.io/crates/mailin) by **Saul Hazledine** is quite similar to samotop:
    * same: recent activity (Mailin last commits: Feb 2020)
    * same: enables writing SMTP servers in Rust.
    * same: includes SMTP parsing, responding and an SMTP state machine.
    * different: Samotop uses PEG, Mailin uses Nom to define the SMTP parser.
    * different: Samotop is async while Mailin runs on bare std blocking IO. Async introduces more dependencies, but allows us to shift to the new IO paradigm. In Samotop, the SMTP session is handled as a stream of commands and responses. Mailin uses a threadpool to schedule work, Samotop can run on a single thread thanks to async.
    * not too different: samotop includes a default TCP server and enables the user to implement it differently, mailin expects the user to provide a socket but a TCP server is available in mailin-embedded. Thanks to this, Mailin alone has much smaller dependency footprint. Samotop may follow suit to split the crates.
    * ...
* [smtpbis](https://crates.io/crates/smtpbis) and [rustyknife](https://crates.io/crates/rustyknife) by **Jonathan Bastien-Filiatrault** are SMTP libraries on async and tokio.
    * same: async.
    * different: Samotop moved to async-std, smtpbis is on tokio.
    * ...
* [rust-smtp](https://github.com/mneumann/rust-smtp) by **mneumann**, last commit 2014, parser coded manually, looks unfinished and abandoned.
* [rust-smtp](https://github.com/synlestidae/rust-smtp) fork of the above with progress by **synlestidae** in 2016

## Other
* [lettre](https://github.com/lettre/lettre) is an SMTP client, it seems to be alive and well!
* [segimap](https://github.com/uiri/SEGIMAP) by **uiri**, that's actually an IMAP server.
* [ferric-mail](https://github.com/wraithan/ferric-mail) by **wraithan**, looks abandoned since 2014.
* [new-tokio-smtp](https://crates.io/crates/new-tokio-smtp) is na SMTP client by **Philipp Korber**, now only pasively maintained
*/

#[macro_use]
extern crate log;

pub mod grammar;
pub mod model;
pub mod protocol;
pub mod server;
pub mod service;

mod common {
    pub use crate::model::{Error, Result};

    //pub use futures::ready;
    pub use bytes::{Bytes, BytesMut};
    pub use futures::{
        future, ready, stream, AsyncRead as Read, AsyncReadExt as ReadExt, AsyncWrite as Write,
        AsyncWriteExt as WriteExt, Future, FutureExt, Sink, Stream, StreamExt, TryFutureExt,
    };
    pub use pin_project::pin_project;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};
}

#[cfg(test)]
pub mod test_util {

    pub use crate::common::*;
    use crate::protocol::MayBeTls;
    use std::collections::VecDeque;

    pub fn cx() -> Context<'static> {
        std::task::Context::from_waker(futures::task::noop_waker_ref())
    }

    pub fn b(bytes: impl AsRef<[u8]>) -> Bytes {
        Bytes::copy_from_slice(bytes.as_ref())
    }

    #[pin_project]
    pub struct TestStream<I> {
        items: VecDeque<Poll<Option<I>>>,
    }
    impl<T: IntoIterator<Item = Poll<Option<I>>>, I> From<T> for TestStream<I> {
        fn from(from: T) -> Self {
            TestStream {
                items: from.into_iter().collect(),
            }
        }
    }
    impl<I> Stream for TestStream<I> {
        type Item = I;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if let Some(item) = self.project().items.pop_front() {
                item
            } else {
                Poll::Ready(None)
            }
        }
    }

    #[pin_project]
    pub struct TestIO {
        pub input: Vec<u8>,
        pub output: Vec<u8>,
        pub read: usize,
        pub read_chunks: VecDeque<usize>,
    }
    impl TestIO {
        pub fn written(&self) -> &[u8] {
            &self.output[..]
        }
        pub fn read(&self) -> &[u8] {
            &self.input[..self.read]
        }
        pub fn unread(&self) -> &[u8] {
            &self.input[self.read..]
        }
        pub fn new() -> Self {
            TestIO {
                output: vec![],
                input: vec![],
                read: 0,
                read_chunks: vec![].into(),
            }
        }
        // Pretend reading chunks of input of given sizes. 0 => Pending
        pub fn add_read_chunk(mut self, chunk: impl AsRef<[u8]>) -> Self {
            self.input.extend_from_slice(chunk.as_ref());
            self.read_chunks.push_back(chunk.as_ref().len());
            self
        }
    }
    impl<T: AsRef<[u8]>> From<T> for TestIO {
        fn from(data: T) -> Self {
            Self::new().add_read_chunk(data)
        }
    }
    impl Read for TestIO {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            let proj = self.project();
            match proj.read_chunks.pop_front() {
                None => Poll::Ready(Ok(0)),
                Some(max) => {
                    let len = usize::min(max, proj.input.len() - *proj.read);
                    let len = usize::min(len, buf.len());
                    if len != max {
                        proj.read_chunks.push_front(max - len);
                    }
                    if len == 0 {
                        Poll::Pending
                    } else {
                        (&mut buf[..len])
                            .copy_from_slice(&proj.input[*proj.read..*proj.read + len]);
                        *proj.read += len;
                        Poll::Ready(Ok(len))
                    }
                }
            }
        }
    }
    impl Write for TestIO {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let proj = self.project();
            proj.output.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    impl MayBeTls for TestIO {
        fn start_tls(self: Pin<&mut Self>) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "TLS not supported",
            ))
        }
        fn supports_tls(&self) -> bool {
            false
        }
    }
}
