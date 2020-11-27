mod command;
mod commands;
pub mod extension;
mod extensions;
mod reply;
mod state;

pub use self::command::*;
pub use self::extensions::*;
pub use self::reply::*;
pub use self::state::*;
use crate::{common::S2Fut, parser::ParseError};
use std::fmt;

/// Represents the instructions for the client side of the stream.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WriteControl {
    /// The stream should be shut down.
    Shutdown(SmtpReply),
    /// Tell codec to start data
    StartData(SmtpReply),
    /// Tell stream to upgrade to TLS
    StartTls(SmtpReply),
    /// Send an SMTP reply
    Reply(SmtpReply),
}

/// Represents the instructions for the server side of the stream.
pub enum ReadControl {
    /** SMTP command line */
    Command(Box<dyn SmtpSessionCommand>, Vec<u8>),
    /** raw input that could not be understood */
    Raw(Vec<u8>),
    /** Available mail data without signalling dots */
    MailDataChunk(Vec<u8>),
    /** The SMTP data terminating dot (. CR LF) is part of protocol signalling and not part of data  */
    EndOfMailData(Vec<u8>),
    /** The SMTP data escape dot (.) is part of protocol signalling and not part of data */
    EscapeDot(Vec<u8>),
    /// Empty line or white space
    Empty(Vec<u8>),
}

impl fmt::Debug for ReadControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        enum TB<'a> {
            T(&'a str),
            B(&'a [u8]),
        }
        fn tb(inp: &[u8]) -> TB {
            if let Ok(text) = std::str::from_utf8(inp) {
                TB::T(text)
            } else {
                TB::B(inp)
            }
        }
        match self {
            ReadControl::Command(c, b) => f
                .debug_tuple("Command")
                .field(&c.verb())
                .field(&tb(b))
                .finish(),
            ReadControl::Raw(b) => f.debug_tuple("Raw").field(&tb(b)).finish(),
            ReadControl::MailDataChunk(b) => f.debug_tuple("MailDataChunk").field(&tb(b)).finish(),
            ReadControl::EndOfMailData(b) => f.debug_tuple("EndOfMailData").field(&tb(b)).finish(),
            ReadControl::EscapeDot(b) => f.debug_tuple("EscapeDot").field(&tb(b)).finish(),
            ReadControl::Empty(b) => f.debug_tuple("Empty").field(&tb(b)).finish(),
        }
    }
}

impl SmtpSessionCommand for ReadControl {
    fn verb(&self) -> &str {
        match self {
            ReadControl::Raw(_) => "",
            ReadControl::Command(cmd, _) => cmd.verb(),
            ReadControl::MailDataChunk(_) => "",
            ReadControl::EndOfMailData(_) => MailBodyEnd.verb(),
            ReadControl::Empty(_) => "",
            ReadControl::EscapeDot(_) => "",
        }
    }

    fn apply<'a>(&'a self, mut state: SmtpState) -> S2Fut<'a, SmtpState> {
        Box::pin(async move {
            if !state.reads.is_empty() {
                // previous raw control left some bytes behind
                match self {
                    ReadControl::Raw(_) => {
                        // ok, parsing will carry on
                    }
                    _ => {
                        // nope, we will not parse the leftover, let's say so.
                        state.reads.clear();
                        state = SmtpInvalidCommand::default().apply(state).await;
                    }
                }
            }

            match self {
                ReadControl::Command(cmd, _) => cmd.apply(state).await,
                ReadControl::MailDataChunk(bytes) => MailBodyChunk(bytes).apply(state).await,
                ReadControl::EndOfMailData(_) => MailBodyEnd.apply(state).await,
                ReadControl::Empty(_) => state,
                ReadControl::EscapeDot(_) => state,
                ReadControl::Raw(b) => {
                    state.reads.extend_from_slice(b.as_slice());

                    loop {
                        break if state.reads.is_empty() {
                            state
                        } else {
                            match state.service.parse_command(state.reads.as_slice()) {
                                Ok((remaining, command)) => {
                                    trace!(
                                        "Parsed {} bytes - a {} command",
                                        state.reads.len() - remaining.len(),
                                        command.verb()
                                    );
                                    state.reads = remaining.to_vec();
                                    state = command.apply(state).await;
                                    continue;
                                }
                                Err(ParseError::Incomplete) => {
                                    // we will need more bytes...
                                    state
                                }
                                Err(e) => {
                                    warn!(
                                        "Parser did not match, passing current line as is {} long. {:?}",
                                        state.reads.len(), e
                                    );
                                    state.reads.clear();
                                    SmtpInvalidCommand::default().apply(state).await
                                }
                            }
                        };
                    }
                }
            }
        })
    }
}
