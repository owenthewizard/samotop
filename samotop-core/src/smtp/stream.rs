use crate::common::*;
use crate::smtp::{ReadControl, SmtpSessionCommand, SmtpState, WriteControl};

#[pin_project(project=SessionProj)]
pub struct SessionStream<S> {
    #[pin]
    input: S,
    state: State<SmtpState>,
}

enum State<T> {
    Ready(T),
    Pending(S2Fut<'static, T>),
    Taken,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self::Taken
    }
}

impl<S> Stream for SessionStream<S>
where
    S: Stream<Item = Result<ReadControl>> + Unpin + Sync + Send,
{
    type Item = Result<WriteControl>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("poll_next");
        loop {
            if let Some(answer) = ready!(self.as_mut().poll_pop(cx)) {
                break Poll::Ready(Some(Ok(answer)));
            }
            let proj = self.as_mut().project();
            break match std::mem::take(proj.state) {
                State::Taken => Poll::Ready(None),
                State::Pending(_) => unreachable!("handled by previous poll"),
                State::Ready(data) => {
                    trace!("poll_next polling input");
                    let res = match proj.input.poll_next(cx) {
                        Poll::Pending => {
                            *proj.state = State::Ready(data);
                            Poll::Pending
                        }
                        Poll::Ready(None) => Poll::Ready(None),
                        Poll::Ready(Some(Ok(control))) => {
                            trace!("poll_next polled input {:?}", control);
                            *proj.state =
                                State::Pending(Box::pin(async move { control.apply(data).await }));
                            continue;
                        }
                        Poll::Ready(Some(Err(e))) => {
                            error!("reading SMTP input failed: {:?}", e);
                            Poll::Ready(Some(Ok(WriteControl::Shutdown(
                                samotop_model::smtp::SmtpReply::ProcesingError,
                            ))))
                        }
                    };
                    trace!("poll_next polled input {:?}", res);
                    res
                }
            };
        }
    }
}

impl<S> SessionStream<S>
where
    S: Stream<Item = Result<ReadControl>> + Unpin + Sync + Send,
{
    pub fn new(input: S, state: SmtpState) -> Self {
        Self {
            state: State::Ready(state),
            input,
        }
    }
    fn poll_pop(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<WriteControl>> {
        trace!("poll_pop");
        let proj = self.project();
        let res = match proj.state {
            State::Taken => Poll::Ready(None),
            State::Ready(ref mut data) => Poll::Ready(data.writes.pop_front()),
            State::Pending(ref mut fut) => {
                let mut data = ready!(fut.as_mut().poll(cx));
                let pop = Poll::Ready(data.writes.pop_front());
                *proj.state = State::Ready(data);
                pop
            }
        };
        trace!("popped {:?}", res);
        res
    }
}
