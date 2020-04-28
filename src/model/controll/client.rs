use crate::model::response::SmtpReply;

/// Represents the instructions towards the client side of the stream.
#[derive(Debug, Clone)]
pub enum ClientControll {
    /// The stream should be shut down.
    Shutdown,
    /// See `ServerControll::ConfirmSwitchToData`
    AcceptData(bool),
    /// Send an SMTP reply
    Reply(SmtpReply),
    /// Something got done, but we should call back again
    Noop,
}
