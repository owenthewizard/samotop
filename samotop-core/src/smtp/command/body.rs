/// Represents parsed mail body parts
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum MailBody<B> {
    /// A chunk of the mail body without a trailing CRLF
    Chunk { data: B, ends_with_new_line: bool },
    /// The mail body is finished. Mail should be queued.
    End,
}
