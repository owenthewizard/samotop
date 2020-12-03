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
use crate::parser::Parser;
use std::fmt;

/// Represents the instructions for the client side of the stream.
pub enum CodecControl {
    /// Write an SMTP response
    Response(Vec<u8>),
    /// Switch parser
    Parser(Box<dyn Parser + Sync + Send>),
    /// Start TLS encryption
    StartTls,
    /// Shut the stream down
    Shutdown,
}

impl fmt::Debug for CodecControl {
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
            CodecControl::Parser(p) => f.debug_tuple("Parser").field(&p).finish(),
            CodecControl::Response(r) => f.debug_tuple("Response").field(&tb(r)).finish(),
            CodecControl::StartTls => f.debug_tuple("StartTls").finish(),
            CodecControl::Shutdown => f.debug_tuple("Shutdown").finish(),
        }
    }
}

// /// Represents the instructions for the server side of the stream.
// pub enum ReadControl {
//     /** SMTP command line */
//     Command(Box<dyn SmtpSessionCommand>, Vec<u8>),
//     /** Available mail data without signalling dots */
//     MailDataChunk(Vec<u8>),
//     /** The SMTP data terminating dot (. CR LF) is part of protocol signalling and not part of data  */
//     EndOfMailData(Vec<u8>),
//     /** The SMTP data escape dot (.) is part of protocol signalling and not part of data */
//     EscapeDot(Vec<u8>),
//     /// Empty line or white space
//     Empty(Vec<u8>),
// }

// impl fmt::Debug for ReadControl {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         #[derive(Debug)]
//         enum TB<'a> {
//             T(&'a str),
//             B(&'a [u8]),
//         }
//         fn tb(inp: &[u8]) -> TB {
//             if let Ok(text) = std::str::from_utf8(inp) {
//                 TB::T(text)
//             } else {
//                 TB::B(inp)
//             }
//         }
//         match self {
//             ReadControl::Command(c, b) => f
//                 .debug_tuple("Command")
//                 .field(&c.verb())
//                 .field(&tb(b))
//                 .finish(),
//             ReadControl::MailDataChunk(b) => f.debug_tuple("MailDataChunk").field(&tb(b)).finish(),
//             ReadControl::EndOfMailData(b) => f.debug_tuple("EndOfMailData").field(&tb(b)).finish(),
//             ReadControl::EscapeDot(b) => f.debug_tuple("EscapeDot").field(&tb(b)).finish(),
//             ReadControl::Empty(b) => f.debug_tuple("Empty").field(&tb(b)).finish(),
//         }
//     }
// }

// impl SmtpSessionCommand for ReadControl {
//     fn verb(&self) -> &str {
//         match self {
//             ReadControl::Command(cmd, _) => cmd.verb(),
//             ReadControl::MailDataChunk(_) => "",
//             ReadControl::EndOfMailData(_) => MailBodyEnd.verb(),
//             ReadControl::Empty(_) => "",
//             ReadControl::EscapeDot(_) => "",
//         }
//     }

//     fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
//         Box::pin(async move {
//             match self {
//                 ReadControl::Command(cmd, _) => cmd.apply(state).await,
//                 ReadControl::MailDataChunk(bytes) => unimplemented!(),
//                 ReadControl::EndOfMailData(_) => MailBodyEnd.apply(state).await,
//                 ReadControl::Empty(_) => state,
//                 ReadControl::EscapeDot(_) => state,
//             }
//         })
//     }
// }
