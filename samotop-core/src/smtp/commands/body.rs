/// A chunk of the mail body
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct MailBodyChunk<B>(pub B);

/// The mail body is finished. Mail should be queued.
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct MailBodyEnd;
