[![Build Status](https://gitlab.com/BrightOpen/Samotop/badges/develop/pipeline.svg)](https://gitlab.com/BrightOpen/Samotop/commits/master)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# samotop 0.13.1+smtp

This is an SMTP server library with focus on privacy.
There is also an actual SMTP server - see
[samotop-server](https://crates.io/crates/samotop-server).

SMTP Server (Relay/MTA, Delivery/MDA) library for Rust
with focus on spam elimination and privacy.
The motivation is to revive e-mail infrastructure
and architecture, address current problems
and allow new systems to integrate SMTP.
It's called SaMoToP, which could be a nice Czech word.

## Status

You can implement your own mail service and plug it in,
focusing on features and not the protocol itself or boilerplate.
The API builds on async/await to offer a convenient asynchronous interface.
We've got a decent SMTP command parser written as a PEG grammar.
The model is tightly nit from the RFCs. An async-std based server
will hear your SMTP commands, drive the SMTP state machine and
correct you if you step aside. Once a mail session is ready,
the mail can be dumped to the console, saved in a folder or passed to a downstream SMTP/LMTP server.
After that, you can do it again. See the [api docs](https://docs.rs/samotop/).
The [samotop crate is published on crates.io](https://crates.io/crates/samotop).

### Done

- [x] Parse SMTP commands and write responses according to RFCs
- [x] SMTP state machine - helo, mail, rcpt*, data, rset, quit - must be in correct order according to RFCs
- [x] DATA are handled and terminated correctly (escape dot, final dot).
- [x] Async/await with async-std backing
- [x] Privacy: TLS/STARTTLS supported using [rustls](https://crates.io/crates/rustls) and [native_tls](https://crates.io/crates/native_tls)
- [x] Privacy: Encryption at rest, S/MIME encrypt e-mails, only the recipient will be able to decrypt
- [x] MTA: Simple mail relay, logging smtp session to standard output but able to receive mail from common relays
- [x] MDA: System-wide mailbox - mailbox for all unclaimed domains / addresses - store mail in a folder so it can be processed further
- [x] MDA: Domain mailbox - mailbox for unclaimed addresses (through LMTP to another LDA)
- [x] MDA: User mailbox - mailbox for specific address or alias (through LMTP to another LDA)
- [x] MDA: Store mail in Maildir (through LMTP to another LDA)
- [x] MDA: Smart mailbox - multiple mailbox addresses by convention
- [x] Integration: LMTP socket - can deliver to LDA over unix or network sockets using LMTP
- [x] Integration: LMTP child process - can deliver to LDA using LMTP protocol over io with a child process
- [x] LDA: Can process LMTP session (LHLO + delivery status per rcpt)
- [x] Antispam: Reject mails failing SPF checks - through `viaspf` crate, now async
- [x] Antispam: Strict SMTP - require CRLF
- [x] Antispam: Strict SMTP - reject session if client sends mail before banner - `Prudence`
- [x] Anti-abuse: Command timeout - `Impatience`
- [x] Extensibility: Modular and composable service - `Builder` + `Configuration` + `MailSetup` => `Service`

### To do

- [ ] Accounts: Self service account subscription through SMTP/IMAP
- [ ] MTA: Queue and queue manager, relay mail to another MTA
- [ ] Antispam: whitelist and blacklist
- [ ] Antispam: greylisting
- [ ] Antispam: white/black/grey list with UI - user decides new contact handling
- [ ] Antispam: is it encrypted?
- [ ] Antispam: reverse lookup
- [ ] Antispam: DANE (DNSSEC) with UI - user verifies signatures
- [ ] Privacy: Refuse unencrypted session
- [ ] Privacy: Leave no trace, no logs, obfuscated file dates...

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies.samotop]
version = "0"
```

## Usage

See the docs on [docs.rs](https://docs.rs/samotop).

Note that the API is still unstable. Please use the latest release.

There are a few interesting provisions one could take away from Samotop:
* The TCP server (`TcpServer`) - it takes IP:port's to listen `on()` and you can then `serve()` your own implementation of a `IoService`.
* The Unix socket server (`UnixServer`) - it takes socket file path to listen `on()` and you can then `serve()` the same as with the `TcpServer`.
* The SMTP session parser (`SmtpParser`) - it takes `&[u8]` and returns parsed commands or data.
* The SMTP session and domain model (in `samotop-core`) - these describe the domain and behavior per RFC.
* The mail delivery abstraction in `samotop-delivery` includes an SMTP/LMTP client over TCP/Unix socket, chid process, simple maildir, sendmail.
* Extensible design - you can plug in or compose your own solution.

### SMTP Server (with STARTTLS)

Running an SMTP server with STARTTLS support is a bit more involved
regarding setting up the TLS configuration. The library includes a `TlsProvider`
implementation for async-tls (rustls) and async-native-tls (native-tls).
The samotop-server is a working reference for this TLS setup
where you need to provide only the cert and key.
You can also implement your own `TlsProvider` and plug it in.

### SMTP Server (plaintext)

You can easily run a plaintext SMTP service without support for STARTTLS.
Replace `Builder` with your own implementation or compose
a mail service with `Builder + mailsetup` and provided features.

Look at samotop-server for a working example with TLS and other features.

```rust
extern crate async_std;
extern crate env_logger;
extern crate samotop;
#[async_std::main]
async fn main() {
    use samotop::mail::*;
    use samotop::smtp::*;
    use samotop::server::*;
    env_logger::init();

    let mail = Builder
                + Esmtp.with(SmtpParser);

    let srv = TcpServer::on("localhost:25").serve(mail.build());

    srv.await.expect("success")
}
```

### TCP server
Any TCP service can be served. See the docs for `IoService`.
Run it with `RUST_LOG=trace` to display trace log.
Use this to understand how networking IO is handled.
Start here to build an SMTP service from scratch step by step.

```rust
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::TcpServer;
use samotop::io::DummyService;
fn main() {
    env_logger::init();
    let mut srv = TcpServer::on("localhost:0").serve(DummyService);
    async_std::task::block_on(srv).unwrap()
}
```

### Unix socket server
You can serve the same on Unix sockets

```rust
extern crate async_std;
extern crate env_logger;
extern crate samotop;
use samotop::server::UnixServer;
use samotop::io::DummyService;
fn main() {
    env_logger::init();
    let mut srv = UnixServer::on("local.socket").serve(DummyService);
    async_std::task::block_on(srv).unwrap()
}
```

### Stdio server
You can serve the same on a command line IO. See the on-cmd example.

## Development

* The usual rustup + cargo setup is required.
* The software is automatically built, tested and published using Gitlab CI/CD pipelines.
* README's are generated manually from rust docs using cargo-readme. Do not modify README's directly:
  ```bash
  $ cargo readme > README.md`
  ```

## Company
In Rust world I have so far found mostly SMTP clients.

### SMTP server implementations and libs
* [mailin](https://crates.io/crates/mailin) by **Saul Hazledine** is quite similar to samotop:
    * same: recent activity (Mailin last commits: Feb 2020)
    * same: enables writing SMTP servers in Rust.
    * same: includes SMTP parsing, responding and an SMTP state machine.
    * different: Samotop uses PEG, Mailin uses Nom to define the SMTP parser.
    * different: Samotop is async while Mailin runs on bare std blocking IO. Async introduces more dependencies, but allows us to shift to the new IO paradigm. In Samotop, the SMTP session handling is a tree of futures. Mailin uses a threadpool to schedule work, Samotop can run on a single thread thanks to async.
    * not too different: samotop includes a default TCP server and enables the user to implement it differently, mailin expects the user to provide a socket but a TCP server is available in mailin-embedded. Thanks to this, Mailin alone has much smaller dependency footprint. Samotop may follow suit to split the crates.
    * ...
* [smtpbis](https://crates.io/crates/smtpbis) and [rustyknife](https://crates.io/crates/rustyknife) by **Jonathan Bastien-Filiatrault** are SMTP libraries on async and tokio.
    * same: async.
    * different: Samotop moved to async-std, smtpbis is on tokio.
    * ...
* [rust-smtp](https://github.com/mneumann/rust-smtp) by **mneumann**, last commit 2014, parser coded manually, looks unfinished and abandoned.
* [rust-smtp](https://github.com/synlestidae/rust-smtp) fork of the above with progress by **synlestidae** in 2016

### Other
* [async-smtp](https://github.com/async-email/async-smtp) is an SMTP client from the awesome [delta.chat](https://delta.chat) project. I've forked it as samotop-delivery.
* [lettre](https://github.com/lettre/lettre) is an SMTP client, it seems to be alive and well!
* [segimap](https://github.com/uiri/SEGIMAP) by **uiri**, that's actually an IMAP server.
* [ferric-mail](https://github.com/wraithan/ferric-mail) by **wraithan**, looks abandoned since 2014.
* [new-tokio-smtp](https://crates.io/crates/new-tokio-smtp) is na SMTP client by **Philipp Korber**, now only pasively maintained

## License

MIT OR Apache-2.0

> :warning: **WARNING: GPLv3** in samotop-parser-nom crate - parser-nom feature of this crate
>
> Since the rustyknife crate is licensed under GPLv3, it's terms apply to you if you distribute a binary including the parser-nom feature.

### Contribution

Unless you explicitly state otherwise, any contribution submitted for inclusion in samotop projects by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.
