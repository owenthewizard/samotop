use model::response::SmtpReply;

#[derive(Debug, Clone)]
pub enum ClientControll {
    Shutdown,
    AcceptData,
    QueueMail,
    Reply(SmtpReply),
    Noop,
}
