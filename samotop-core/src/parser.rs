use crate::{
    common::*,
    mail::{Configuration, MailSetup, Rfc5321},
    smtp::{SmtpSessionCommand, SmtpState, SmtpUnknownCommand},
};
use std::{
    fmt::{self, Debug, Display},
    marker::PhantomData,
    ops::Deref,
};

pub type ParseResult<'a, T> = std::result::Result<(&'a [u8], T), ParseError>;

pub trait Parser: fmt::Debug {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>>;
}

impl<T> Parser for Arc<T>
where
    T: Parser,
{
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        T::parse_command(self, input)
    }
}

impl Parser for Vec<Arc<dyn Parser + Sync + Send>> {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        for (idx, parser) in self.iter().enumerate() {
            trace!("Parser {} parse_command calling {:?}", idx, parser);
            match parser.parse_command(input) {
                Err(ParseError::Mismatch(e)) => {
                    debug!(
                        "Parser {} - {:?} did not recognize the input: {:?}",
                        idx, parser, e
                    );
                }
                otherwise => return otherwise,
            }
        }
        Err(ParseError::Mismatch("No parser can parse this".into()))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Dummy;

#[derive(Debug)]
pub enum ParseError {
    Incomplete,
    Failed(Error),
    Mismatch(Error),
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Incomplete => write!(f, "The input is not complete"),
            ParseError::Failed(e) => write!(f, "The input is invalid, parsing failed: {}", e),
            ParseError::Mismatch(e) => write!(f, "Parser did not match the input: {}", e),
        }
    }
}
impl std::error::Error for ParseError {}

/// Dummy parser implementation
impl Parser for () {
    /// Produces unknown command for each new line
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        if let Some(line) = input.split(|b| *b == b'\n').next() {
            Ok((
                &input[line.len() + 1..],
                Box::new(Rfc5321::command(SmtpUnknownCommand::default())),
            ))
        } else {
            Err(ParseError::Incomplete)
        }
    }
}

pub type ParseResult2<T> = std::result::Result<(usize, T), ParseError>;

pub trait Parser3<CMD>: fmt::Debug {
    fn parse(&self, input: &[u8], state: &SmtpState) -> ParseResult2<CMD>;
}

impl Parser3<SmtpUnknownCommand> for () {
    fn parse(&self, input: &[u8], _state: &SmtpState) -> ParseResult2<SmtpUnknownCommand> {
        if let Some(line) = input.split(|b| *b == b'\n').next() {
            Ok((line.len(), Default::default()))
        } else {
            Err(ParseError::Incomplete)
        }
    }
}

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

#[derive(Debug)]
pub struct DummyError {}

impl Display for DummyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Dummy parser error")
    }
}
impl std::error::Error for DummyError {}

pub trait Action<CMD> {
    fn apply<'a, 'c, 's, 'f>(&'a self, cmd: CMD, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'c: 'f,
        's: 'f;
}

impl<CMD> Action<CMD> for () {
    fn apply<'a, 'c, 's, 'f>(&'a self, _cmd: CMD, _state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'c: 'f,
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
        Box::pin(ready(Err(ParseError::Mismatch(Box::new(DummyError {})))))
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
        todo!()
    }
}

pub type InterpretResult = std::result::Result<usize, ParseError>;

pub struct Interpretter {
    calls: Vec<Box<dyn Interpret + Send + Sync>>,
}

impl MailSetup for Interpretter {
    fn setup(mut self, config: &mut Configuration) {
        self.calls.push(config.interpretter.clone());
        config.interpretter = Box::new(Arc::new(self))
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
        P: Parser3<CMD> + Debug + 'static + Send + Sync,
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
            return todo!("failures");
        } else if incomplete {
            return todo!("incomplete");
        } else if !mismatches.is_empty() {
            return todo!("mismatches");
        } else {
            return todo!("no match");
        }
    }
}
impl Debug for Interpretter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Interpretter({})", self.calls.len()))
    }
}
impl Default for Interpretter {
    fn default() -> Self {
        Interpretter { calls: vec![] }
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
        P: Parser3<CMD> + 'static + Send + Sync,
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
        P: Parser3<CMD> + 'static + Send + Sync,
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
    P: Parser3<CMD> + 'static + Send + Sync,
    A: Action<CMD> + 'static + Send + Sync,
    CMD: 'static + Send + Sync,
{
    async fn perform_inner(&self, input: &[u8], state: &mut SmtpState) -> InterpretResult {
        let (length, cmd) = self.parser.parse(input, state)?;
        self.action.apply(cmd, state).await;
        Ok(length)
    }
}
impl<CMD, P, A> Interpret for ParserAction<P, A, CMD>
where
    P: Parser3<CMD> + Debug + 'static + Send + Sync,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpretter_handle_test() {
        let parser = ();
        let action = ();
        assert_eq!(
            format!(
                "{:#?}",
                Interpretter::default().handle::<_, _, SmtpUnknownCommand>(parser, action)
            ),
            format!(
                "{:#?}",
                Interpretter {
                    calls: vec![Box::new(ParserAction {
                        parser,
                        action,
                        phantom: PhantomData::<SmtpUnknownCommand>
                    })]
                }
            ),
        );
    }
    #[test]
    fn builder_parse_with_apply_test() {
        let parser = ();
        let action = ();
        assert_eq!(
            format!(
                "{:#?}",
                Interpretter::default()
                    .parse::<SmtpUnknownCommand>()
                    .with(parser)
                    .and_apply(action)
            ),
            format!(
                "{:#?}",
                Interpretter {
                    calls: vec![Box::new(ParserAction {
                        parser,
                        action,
                        phantom: PhantomData::<SmtpUnknownCommand>
                    })]
                }
            ),
        );
    }
}
