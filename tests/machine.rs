extern crate samotop;
#[macro_use]
extern crate futures;
extern crate tokio;

use samotop::model::command::SmtpCommand;
use samotop::model::controll::{ClientControll, ServerControll};
use samotop::model::session::Session;
use samotop::util::*;
use tokio::prelude::*;

#[test]
fn machine_test() {
    let controls = vec![ServerControll::Command(SmtpCommand::Data)];

    let stream = stream::iter_ok(controls.into_iter());

    let task = stream
        .machine("Howdy!")
        .for_each(|c| Ok(println!("{:?}", c)));

    tokio::run(task);
}

pub trait IntoMachine
where
    Self: Sized,
{
    fn machine(self, name: impl ToString) -> Machine<Self> {
        Machine::new(self, name.to_string())
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
                println!("Answer: {:?}", a);
                return ok(a);
            }
            _ => {}
        };

        match try_ready!(self.stream.poll()) {
            None => none(),
            Some(ctrl) => {
                println!("Controll: {:?}", ctrl);
                self.state.controll(ctrl);
                match self.state.answer() {
                    Some(a) => {
                        println!("Answer: {:?}", a);
                        ok(a)
                    }
                    None => pending(),
                }
            }
        }
    }
}
