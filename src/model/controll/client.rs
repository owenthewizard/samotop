use model::response::SmtpReply;

#[derive(Debug, Clone)]
pub enum ClientControll {
    Shutdown,
    AcceptData,
    Reply(SmtpReply),
    /// Something got done, but we should call back again
    Noop,
}
