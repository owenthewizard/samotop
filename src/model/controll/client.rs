
#[derive(Debug)]
pub enum ClientControll {
    Shutdown,
    AcceptData,
    Reply(String),
}
