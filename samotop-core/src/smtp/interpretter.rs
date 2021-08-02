use crate::{
    common::*,
    smtp::{ParseError, Parser, SmtpState},
};
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::Deref,
};
pub trait Interpret: Debug {
    fn interpret<'a, 's, 'f>(&'a self, state: &'s mut SmtpState) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f;
}

/// An action modifies the SMTP state based on some command
pub trait Action<CMD> {
    fn apply<'a, 's, 'f>(&'a self, cmd: CMD, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f;
}

impl<CMD: Send + 'static> Action<CMD> for Dummy {
    fn apply<'a, 's, 'f>(&'a self, _cmd: CMD, _state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(()))
    }
}

impl Interpret for Dummy {
    fn interpret<'a, 'i, 's, 'f>(&'a self, _state: &'s mut SmtpState) -> S1Fut<'f, InterpretResult>
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
    fn interpret<'a, 'i, 's, 'f>(&'a self, state: &'s mut SmtpState) -> S1Fut<'f, InterpretResult>
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
    fn interpret<'a, 'i, 's, 'f>(&'a self, state: &'s mut SmtpState) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(interpret_all(self.calls.as_slice(), state))
    }
}
impl Interpretter {
    pub fn new(calls: Vec<Box<dyn Interpret + Send + Sync>>) -> Self {
        Self { calls }
    }
    pub fn parse<CMD>(self) -> InterpretterBuilderCommand<CMD>
    where
        CMD: 'static + Send + Sync,
    {
        InterpretterBuilderCommand {
            inner: self,
            phantom: PhantomData,
        }
    }
    pub fn handle<P, A, CMD>(self, parser: P, action: A) -> Self
    where
        P: Parser<CMD> + Debug + 'static + Send + Sync,
        A: Action<CMD> + Debug + 'static + Send + Sync,
        CMD: Debug + 'static + Send + Sync,
    {
        let call = ParserAction {
            parser,
            action,
            phantom: PhantomData,
        };
        self.call(call)
    }
    pub fn call<I: Interpret + Send + Sync + 'static>(mut self, call: I) -> Self {
        self.calls.push(Box::new(call));
        self
    }
}
impl Debug for Interpretter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Interpretter({})", self.calls.len()))
    }
}

pub(crate) async fn interpret_all(
    calls: &[Box<dyn Interpret + Send + Sync>],
    state: &mut SmtpState,
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
pub struct InterpretterBuilderCommand<CMD> {
    inner: Interpretter,
    phantom: PhantomData<CMD>,
}
pub struct InterpretterBuilderParser<P, CMD> {
    inner: Interpretter,
    parser: P,
    phantom: PhantomData<CMD>,
}

impl<CMD> InterpretterBuilderCommand<CMD> {
    pub fn with<P>(self, parser: P) -> InterpretterBuilderParser<P, CMD>
    where
        P: Parser<CMD> + 'static + Send + Sync,
        CMD: 'static + Send + Sync,
    {
        let Self { inner, phantom } = self;
        InterpretterBuilderParser {
            inner,
            parser,
            phantom,
        }
    }
}

impl<P, CMD> InterpretterBuilderParser<P, CMD> {
    pub fn and_apply<A>(self, action: A) -> Interpretter
    where
        A: Action<CMD> + Debug + 'static + Send + Sync,
        P: Parser<CMD> + 'static + Send + Sync,
        CMD: Debug + 'static + Send + Sync,
    {
        let Self {
            inner,
            parser,
            phantom: _,
        } = self;
        inner.handle(parser, action)
    }
}

#[derive(Debug, Clone)]
struct ParserAction<P, A, CMD> {
    parser: P,
    action: A,
    phantom: PhantomData<CMD>,
}

impl<CMD, P, A> ParserAction<P, A, CMD>
where
    P: Parser<CMD> + 'static + Send + Sync,
    A: Action<CMD> + 'static + Send + Sync,
    CMD: 'static + Send + Sync,
{
    async fn perform_inner(&self, state: &mut SmtpState) -> InterpretResult {
        let (length, cmd) = self.parser.parse(state.session.input.as_slice(), state)?;
        self.action.apply(cmd, state).await;
        Ok(Some(length))
    }
}
impl<CMD, P, A> Interpret for ParserAction<P, A, CMD>
where
    P: Parser<CMD> + Debug + 'static + Send + Sync,
    A: Action<CMD> + Debug + 'static + Send + Sync,
    CMD: Debug + 'static + Send + Sync,
{
    fn interpret<'a, 's, 'f>(&'a self, state: &'s mut SmtpState) -> S1Fut<'f, InterpretResult>
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
    use crate::smtp::command::SmtpUnknownCommand;

    #[test]
    fn interpretter_session_setup_test() {
        insta::assert_debug_snapshot!(
            Interpretter::default(),
            @"Interpretter(0)");
    }
    #[test]
    fn interpretter_handle_test() {
        insta::assert_debug_snapshot!(
            Interpretter::default().handle::<_, _, SmtpUnknownCommand>(Dummy, Dummy),
            @"Interpretter(1)");
    }
    #[test]
    fn builder_parse_with_apply_test() {
        insta::assert_debug_snapshot!(Interpretter::default()
            .parse::<SmtpUnknownCommand>()
            .with(Dummy)
            .and_apply(Dummy),
            @"Interpretter(1)");
    }
}
