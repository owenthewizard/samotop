use model::response::SmtpReply;

#[derive(Debug, Clone)]
pub enum ClientControll {
    Shutdown,
    AcceptData,
    #[deprecated(since="0.6.0", note="It will be removed")]
    QueueMail,
    Reply(SmtpReply),
    /// Something got done, but we should call back again
    Noop,
}
