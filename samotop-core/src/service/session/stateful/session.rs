use super::SessionHandler;
use super::SessionState;
use crate::common::*;
use crate::model::io::*;

#[pin_project(project=SessionProj)]
pub struct StatefulSession<I, H: SessionHandler> {
    #[pin]
    input: I,
    handler: H,
    state: Option<State<H::Data>>,
}

type State<T> = SessionState<T>;

impl<I, H> Stream for StatefulSession<I, H>
where
    I: Stream<Item = Result<ReadControl>>,
    H: SessionHandler,
{
    type Item = Result<WriteControl>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("polling next");
        loop {
            ready!(self.as_mut().poll_pending(cx))?;

            if let Some(answer) = self.as_mut().pop_answer() {
                break Poll::Ready(Some(Ok(answer)));
            } else {
                ready!(self.as_mut().poll_input(cx))?;
            }
        }
    }
}

impl<I, H: SessionHandler> StatefulSession<I, H> {
    pub fn new(input: I, handler: H) -> Self {
        Self {
            state: Some(State::Ready(H::Data::default())),
            handler,
            input,
        }
    }
    fn poll_pending(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        trace!("polling pending");
        let SessionProj { state, .. } = self.project();
        match state.as_mut().expect("state must be set") {
            State::Ready(_) => (),
            State::Pending(ref mut fut) => {
                *state = Some(State::Ready(ready!(fut.as_mut().poll(cx))))
            }
        }
        Poll::Ready(Ok(()))
    }
    fn pop_answer(self: Pin<&mut Self>) -> Option<WriteControl> {
        trace!("popping answer");
        let SessionProj { handler, state, .. } = self.project();
        match state.as_mut().expect("state must be set") {
            State::Pending(_) => None,
            State::Ready(ref mut data) => handler.pop(data),
        }
    }
}
impl<I, H> StatefulSession<I, H>
where
    I: Stream<Item = Result<ReadControl>>,
    H: SessionHandler,
{
    fn poll_input(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        trace!("polling input");
        let SessionProj {
            input,
            state,
            handler,
        } = self.project();
        match state.take().expect("state must be set") {
            State::Pending(s) => {
                *state = Some(State::Pending(s));
                Poll::Ready(Ok(()))
            }
            State::Ready(data) => match input.poll_next(cx)? {
                Poll::Ready(None) => {
                    *state = Some(handler.handle(data, ReadControl::PeerShutdown));
                    Poll::Ready(Ok(()))
                }
                Poll::Ready(Some(control)) => {
                    *state = Some(handler.handle(data, control));
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => {
                    *state = Some(State::Ready(data));
                    Poll::Pending
                }
            },
        }
    }
}
