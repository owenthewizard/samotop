use crate::{
    common::*,
    mail::{Configuration, MailSetup},
    smtp::{command::SessionSetup, Dummy, ParseError, Parser, SmtpState},
};
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::Deref,
};
pub trait Interpret: Debug {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f;
}

//#[async_trait::async_trait]
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
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        _input: &'i [u8],
        _state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
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
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Deref::deref(self).interpret(input, state)
    }
}

pub type InterpretResult = std::result::Result<Option<usize>, ParseError>;

pub struct Interpretter {
    calls: Vec<Box<dyn Interpret + Send + Sync>>,
}

impl MailSetup for Interpretter {
    fn setup(mut self, config: &mut Configuration) {
        self.calls.push(Box::new(config.interpretter.clone()));
        config.interpretter = Arc::new(self)
    }
}
impl Interpret for Interpretter {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(self.interpret_inner(input, state))
    }
}
impl Interpretter {
    pub fn session_setup<A>(action: A) -> Self
    where
        A: Action<SessionSetup> + Debug + 'static + Send + Sync,
    {
        let mut this = Interpretter { calls: vec![] };
        this.calls.push(Box::new(SessionSetupAction { action }));
        this
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
    pub fn handle<P, A, CMD>(mut self, parser: P, action: A) -> Self
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
        self.calls.push(Box::new(call));
        self
    }
    async fn interpret_inner(&self, input: &[u8], state: &mut SmtpState) -> InterpretResult {
        let mut mismatches = vec![];
        let mut failures = vec![];
        let mut incomplete = false;
        for call in self.calls.as_slice() {
            match call.interpret(input, state).await {
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
}
impl Debug for Interpretter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Interpretter({})", self.calls.len()))
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
    async fn perform_inner(&self, input: &[u8], state: &mut SmtpState) -> InterpretResult {
        let (length, cmd) = self.parser.parse(input, state)?;
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
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(self.perform_inner(input, state))
    }
}

#[derive(Debug, Clone)]
struct SessionSetupAction<A> {
    action: A,
}
impl<A> Interpret for SessionSetupAction<A>
where
    A: Action<SessionSetup> + Debug + 'static + Send + Sync,
{
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        _input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if !state.session.has_been_set_up {
                self.action.apply(SessionSetup, state).await;
                state.session.has_been_set_up = true;
                Ok(None)
            } else {
                Err(ParseError::Mismatch("Session is already set up".into()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp::command::SmtpUnknownCommand;

    #[test]
    fn interpretter_session_setup_test() {
        insta::assert_debug_snapshot!(
            Interpretter::session_setup(Dummy), 
            @"Interpretter(1)");
    }
    #[test]
    fn interpretter_handle_test() {
        insta::assert_debug_snapshot!(
            Interpretter::session_setup(Dummy).handle::<_, _, SmtpUnknownCommand>(Dummy, Dummy), 
            @"Interpretter(2)");
    }
    #[test]
    fn builder_parse_with_apply_test() {
        insta::assert_debug_snapshot!(Interpretter::session_setup(Dummy)
            .parse::<SmtpUnknownCommand>()
            .with(Dummy)
            .and_apply(Dummy), 
            @"Interpretter(2)");
    }
}
