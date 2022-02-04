use crate::{
    common::*,
    smtp::{ParseError, Parser, SmtpContext},
};
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::Deref,
};
pub trait Interpret: Debug {
    fn interpret<'a, 's, 'f>(&'a self, state: &'s mut SmtpContext) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f;
}

/// An action modifies the SMTP state based on some command
pub trait Action<CMD> {
    fn apply<'a, 's, 'f>(&'a self, cmd: CMD, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f;
}

impl<CMD: Send + 'static> Action<CMD> for Dummy {
    fn apply<'a, 's, 'f>(&'a self, _cmd: CMD, _state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(()))
    }
}

impl Interpret for Dummy {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        _state: &'s mut SmtpContext,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(Err(ParseError::Mismatch("Dummy".into()))))
    }
}
impl<T: Deref> Interpret for T
where
    T::Target: Interpret,
    T: Debug,
{
    fn interpret<'a, 'i, 's, 'f>(&'a self, state: &'s mut SmtpContext) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Deref::deref(self).interpret(state)
    }
}

pub type InterpretResult = std::result::Result<Option<usize>, ParseError>;

#[derive(Default)]
pub struct Interpretter {
    calls: Vec<Box<dyn Interpret + Send + Sync>>,
}

impl Interpret for Interpretter {
    fn interpret<'a, 'i, 's, 'f>(&'a self, state: &'s mut SmtpContext) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(interpret_all(self.calls.as_slice(), state))
    }
}
impl Interpretter {
    pub fn apply<A>(action: A) -> InterpretterBuilderDefault<A> {
        InterpretterBuilderDefault {
            inner: Interpretter::default(),
            action,
        }
    }
    pub fn new(calls: Vec<Box<dyn Interpret + Send + Sync>>) -> Self {
        Self { calls }
    }
}
impl Debug for Interpretter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Interpretter({})", self.calls.len()))
    }
}

pub struct InterpretterBuilder<A> {
    inner: Interpretter,
    action: A,
}
pub struct InterpretterBuilderDefault<A> {
    inner: Interpretter,
    action: A,
}
impl<A> InterpretterBuilderDefault<A> {
    pub fn to<CMD>(self) -> InterpretterBuilder<A>
    where
        A: Action<CMD> + Clone + Debug + 'static + Send + Sync,
        CMD: Debug + 'static + Send + Sync,
    {
        let Self { inner, action } = self;
        let builder = InterpretterBuilder { inner, action };
        builder.to::<CMD>()
    }
}
impl<A> InterpretterBuilder<A> {
    pub fn to<CMD>(mut self) -> Self
    where
        A: Action<CMD> + Clone + Debug + 'static + Send + Sync,
        CMD: Debug + 'static + Send + Sync,
    {
        self.inner.calls.push(Box::new(ParserAction {
            action: self.action.clone(),
            phantom: PhantomData::<CMD>,
        }));
        self
    }
    pub fn apply<A2>(self, action: A2) -> InterpretterBuilderDefault<A2> {
        InterpretterBuilderDefault {
            inner: self.inner,
            action,
        }
    }
    pub fn build(self) -> Interpretter {
        self.inner
    }
}

impl<T> From<InterpretterBuilder<T>> for Interpretter {
    fn from(builder: InterpretterBuilder<T>) -> Self {
        builder.build()
    }
}

pub(crate) async fn interpret_all(
    calls: &[Box<dyn Interpret + Send + Sync>],
    state: &mut SmtpContext,
) -> InterpretResult {
    let mut mismatches = vec![];
    let mut failures = vec![];
    let mut incomplete = false;
    for call in calls {
        match call.interpret(state).await {
            Ok(len) => return Ok(len),
            Err(ParseError::Mismatch(e)) => {
                mismatches.push(e);
                continue;
            }
            Err(ParseError::Incomplete) => {
                incomplete = true;
                continue;
            }
            Err(ParseError::Failed(e)) => {
                failures.push(e);
                continue;
            }
        }
    }
    if !failures.is_empty() {
        let msg = failures.join("; ");
        Err(ParseError::Failed(msg))
    } else if incomplete {
        Err(ParseError::Incomplete)
    } else if !mismatches.is_empty() {
        let msg = mismatches.join("; ");
        Err(ParseError::Mismatch(msg))
    } else {
        Err(ParseError::Mismatch("No parsers?".into()))
    }
}

pub type ParserService<T> = Box<dyn Parser<T> + Sync + Send>;

#[derive(Debug, Clone)]
struct ParserAction<A, CMD> {
    action: A,
    phantom: PhantomData<CMD>,
}

impl<CMD, A> ParserAction<A, CMD>
where
    A: Action<CMD> + 'static + Send + Sync,
    CMD: 'static + Send + Sync,
{
    async fn perform_inner(&self, state: &mut SmtpContext) -> InterpretResult {
        let parser = state
            .get::<ParserService<CMD>>()
            .ok_or_else(|| ParseError::Mismatch("no parser for given CMD".into()))?;

        let (length, cmd) = parser.parse(state.session.input.as_slice(), state)?;
        self.action.apply(cmd, state).await;
        Ok(Some(length))
    }
}
impl<CMD, A> Interpret for ParserAction<A, CMD>
where
    A: Action<CMD> + Debug + 'static + Send + Sync,
    CMD: Debug + 'static + Send + Sync,
{
    fn interpret<'a, 's, 'f>(&'a self, state: &'s mut SmtpContext) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(self.perform_inner(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Dummy;
    use crate::smtp::command::{SmtpInvalidCommand, SmtpUnknownCommand};

    #[test]
    fn interpretter_session_setup_test() {
        insta::assert_debug_snapshot!(
            Interpretter::default(),
            @"Interpretter(0)");
    }
    #[test]
    fn interpretter_handle_test() {
        insta::assert_debug_snapshot!(
            Interpretter::apply(Dummy).to::<SmtpUnknownCommand>().build(),
            @"Interpretter(1)");
    }
    #[test]
    fn builder_parse_with_apply_test() {
        insta::assert_debug_snapshot!(
            Interpretter::apply(Dummy)
                .to::<SmtpUnknownCommand>().build(),
            @"Interpretter(1)");
    }

    #[async_std::test]
    async fn fail_without_parser() {
        let interpretter = Interpretter::apply(Dummy)
            .to::<SmtpInvalidCommand>()
            .build();
        let mut state = SmtpContext::default();
        state.session.input = b"XYZ\r\n".to_vec();
        let res = interpretter.interpret(&mut state).await;
        insta::assert_debug_snapshot!(res, @r###"
        Err(
            Mismatch(
                "no parser for given CMD",
            ),
        )
        "###);
    }
    #[async_std::test]
    async fn interpret_dummy() {
        let interpretter = Interpretter::apply(Dummy)
            .to::<SmtpInvalidCommand>()
            .build();
        let mut state = SmtpContext::default();
        state.set::<ParserService<SmtpInvalidCommand>>(Box::new(Dummy));
        state.session.input = b"XYZ\r\n".to_vec();
        let res = interpretter.interpret(&mut state).await;
        insta::assert_debug_snapshot!(res, @r###"
        Ok(
            Some(
                5,
            ),
        )
        "###);
    }
}
