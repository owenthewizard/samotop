/*!
This is an SMTP server library with focus on privacy.
There is also an actual SMTP server - see
[samotop-server](https://crates.io/crates/samotop-server).

SMTP Server (Relay/MTA, Delivery/MDA) library for Rust
with focus on spam elimination and privacy.
The motivation is to revive e-mail infrastructure
and architecture, address current problems
and allow new systems to integrate SMTP.
It's called SaMoToP, which could be a nice Czech word.

# Status

Reaching stable. You can implement your own mail service and plug it in,
focusing on features and not the protocol itself or boilerplate.
The API builds on async/await to offer a convenient asynchronous interface.
We've got a decent SMTP command parser written as a PEG grammar.
The model is tightly nit from the RFCs. An async-std based server
will hear your SMTP commands, drive the SMTP state machine and
correct you if you step aside. Once a mail session is ready,
the mail data are currently dumped to the console. After that,
you can do it again. See the [api dosc](https://docs.rs/samotop/).
The [samotop crate is published on crates.io](https://crates.io/crates/samotop).

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

# Installation

Add this to your `Cargo.toml`:

```toml
[dependencies.samotop]
version = "0"
```

# Usage

See the docs on [docs.rs](https://docs.rs/samotop).

Note that the API is still unstable. Please use the latest release.

There are a few interesting provisions one could take away from Samotop:
* The server (through `samotop::server::Server`) - it takes IP:port's to listen `on()` and you can then `serve()` your own implementation of a `TcpService`.
* The SMTP service (`SmtpService`) - it takes an async IO and provides an SMTP service defined by `SessionService`.
* The low level `SmtpCodec` - it translates between IO and a `Stram` of `ReadControl` and a `Sink` of `WriteControl`. It handles SMTP mail data as well.
* The SMTP session parser (`SmtpParser`) - it takes `&[u8]` and returns parsed commands or session.
* The SMTP session and domain model (in `samotop::model`) - these describe the domain and behavior.
* Extensible design - you can plug in or compose your own solution.

## SMTP Server (with STARTTLS)

Running an SMTP server with STARTTLS support is a bit more involved
regarding setting up the TLS configuration. The library includes
a `TlsProvider` implementation for async-tls and rustls.
The samotop-server is a working reference for this TLS setup
where you needto provide only the cert and key.
You can also implement your own `TlsProvider` and plug it in.

## SMTP Server (plaintext)

You can easily run a plaintext SMTP service without support for STARTTLS.
Replace `DefaultMailService` with your own implementation or compose
a mail service with `CompositeMailService` and provided features.

```no_run
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::Server;
use samotop::service::tcp::dummy::DummyTcpService;
fn main() {
    env_logger::init();
    let mail = samotop::service::mail::default::DefaultMailService::default();
    let parser = samotop::service::parser::SmtpParser;
    let svc = samotop::service::tcp::smtp::SmtpService::new(mail, parser);
    let svc = samotop::service::tcp::tls::TlsEnabled::disabled(svc);
    let srv = samotop::server::Server::on("localhost:25").serve(svc);
    async_std::task::block_on(srv).unwrap()
}
```

## Dummy server
Any TCP service can be served. See the docs for `TcpService`.
Run it with `RUST_LOG=trace` to display trace log.
Use this to understand how networking IO is handled.
Start here to build an SMTP service from scratch step by step.

```no_run
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::Server;
use samotop::service::tcp::dummy::DummyTcpService;
fn main() {
    env_logger::init();
    let mut srv = Server::on("localhost:0").serve(DummyTcpService);
    async_std::task::block_on(srv).unwrap()
}
```

# Development

* The usual rustup + cargo setup is required.
* The software is automatically built, tested and published using Gitlab CI/CD pipelines.
* README's are generated manually from rust docs using cargo-readme. Do not modify README's directly:
  ```bash
  $ cargo readme > README.md`
  ```

# Company
In Rust world I have so far found mostly SMTP clients.

## SMTP server implementations and libs
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

pub mod protocol{
    pub use samotop_core::protocol::*;
}
pub mod server;
pub mod service;

pub mod model {
    pub use samotop_core::model::*;
}
mod common {
    pub use crate::service::Provider;
    pub use bytes::{Bytes, BytesMut};
    pub use samotop_core::common::*;
}

#[cfg(test)]
pub use samotop_core::test_util;
