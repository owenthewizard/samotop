//! # Status
//!
//! The API is still very much subject to change. Until you see the release of version 1.0.0, don't expect much stability.
//! See the README.md file and project open issues for current status.
//!
//! The use case of running the server as a standalone application should be described in the README.md (tbd)
//! Here we focus on using the library.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! samotop = "0"
//! ```
//!
//! Note that the API is still unstable. Please use the latest release.
//! 
//! # Usage
//!
//! There are a few interesting provisions one could take away here:
//! * The server (through `samotop::builder()`) - it takes IP:port's to listen `on()` and you can use it `with()` your own implementation of `TcpService`.
//! * The SMTP service (`SamotopService`) - it takes a `tokio::net::TcpStream` into the `Sink` created by `start()`.
//! * The low level `SmtpCodec` - it implements `tokio_codec::Encoder` and `tokio_codec::Decoder`. It handles SMTP mail data as well.
//! * The SMTP session parser (`SmtpParser`) - it takes `&str` and returns parsed commands or session.
//! * The SMTP session and domain model (`model::session`, `model::command`, `model::response`) - these describe the domain and behavior.
//! * The mail handling stuff that is yet to be written (`MailService`)...
//!
//! The individual components may later be split out into their own crates, but we shall have the samotop crate re-export them then.
//!
//! # Builder
//! The simplest way is to run the server with a builder:
//!
//! ```no_run
//! extern crate env_logger;
//! extern crate samotop;
//! extern crate tokio;
//! #[macro_use]
//! extern crate structopt;
//!
//! use structopt::StructOpt;
//!
//! fn main() {
//!     env_logger::init();
//!
//!     let opt = Opt::from_args();
//!
//!     tokio::run(samotop::builder()
//!             .on_all(opt.ports)
//!             .as_task());
//! }
//!
//! #[derive(StructOpt, Debug)]
//! #[structopt(name = "samotop")]
//! struct Opt {
//!     /// SMTP server address:port
//!     #[structopt(short = "p", long = "port")]
//!     ports: Vec<String>,
//! }
//! ```

#[macro_use]
extern crate log;
extern crate bytes;
extern crate env_logger;
extern crate regex;
#[macro_use]
extern crate futures;
extern crate hostname;
extern crate tokio;
extern crate tokio_codec;
extern crate uuid;

pub mod grammar;
pub mod model;
pub mod protocol;
pub mod server;
pub mod service;
pub mod util;

pub use server::builder;
