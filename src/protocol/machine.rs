use model::controll::{ClientControll, ServerControll};
use model::session::Session;
use tokio::prelude::*;
use util::futu::*;

pub trait IntoMachine
where
    Self: Sized,
{
    fn machine(self, name: String) -> Machine<Self> {
        Machine::new(self, name)
    }
}

impl<S> IntoMachine for S
where
    S: Stream,
{
}

pub struct Machine<S> {
    stream: S,
    state: Session,
}

impl<S> Machine<S> {
    pub fn new(stream: S, name: String) -> Self {
        let mut me = Self {
            stream,
            state: Session::new(),
        };
        me.state.set_name(name);
        me
    }
}

impl<S> Stream for Machine<S>
where
    S: Stream<Item = ServerControll>,
{
    type Item = ClientControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.state.answer() {
            Some(a) => {
                trace!("Answer: {:?}", a);
                return ok(a);
            }
            _ => {}
        };

        match try_ready!(self.stream.poll()) {
            None => none(),
            Some(ctrl) => {
                trace!("Controll: {:?}", ctrl);
                self.state.controll(ctrl);
                match self.state.answer() {
                    Some(a) => {
                        trace!("Answer: {:?}", a);
                        ok(a)
                    }
                    None => pending(),
                }
            }
        }
    }
}
