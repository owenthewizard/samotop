use samotop_core::{
    parser::Parser,
    smtp::{CodecControl, SmtpSessionCommand},
};

#[derive(Debug)]
pub(crate) struct SwitchParser<C, P> {
    pub command: C,
    pub parser: P,
}

impl<C, P> SmtpSessionCommand for SwitchParser<C, P>
where
    C: SmtpSessionCommand,
    P: Parser + Sync + Send + Clone + 'static,
{
    fn verb(&self) -> &str {
        self.command.verb()
    }

    fn apply(
        &self,
        mut state: samotop_core::smtp::SmtpState,
    ) -> samotop_core::common::S1Fut<samotop_core::smtp::SmtpState> {
        Box::pin(async move {
            state.say(CodecControl::Parser(Box::new(self.parser.clone())));
            state = self.command.apply(state).await;
            state
        })
    }
}
