use model::response::SmtpReply;

#[derive(Debug, Clone)]
pub enum ClientControll {
    Shutdown,
    AcceptData,
    Reply(SmtpReply),
    Noop,
}
