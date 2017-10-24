pub mod codec;
pub mod parser;
pub mod writer;
pub mod socket;
pub mod transport;
mod grammar;

use std::io;
use bytes::Bytes;
use tokio_proto::streaming::pipeline::Frame;
use model::request::SmtpCommand;
use model::response::SmtpReply;

pub type Error = io::Error;
pub type CmdFrame = Frame<SmtpCommand, Bytes, Error>;
pub type RplFrame = Frame<SmtpReply, SmtpReply, Error>;
