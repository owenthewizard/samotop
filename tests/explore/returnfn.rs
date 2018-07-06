use futures::StartSend;
use tokio;
use tokio::io;
use tokio::prelude::*;

struct Subject {
    name: String,
}

trait Svc {
    type Receiver;
    type Error;
    fn start(&self) -> Self::Receiver;
}

struct MySvc {
    name: String,
}

impl Svc for MySvc {
    type Receiver = MyReceiver;
    type Error = io::Error;
    fn start(&self) -> Self::Receiver {
        MyReceiver::new(&self.name)
    }
}

struct MyReceiver {
    name: String,
    pending: Box<Future<Item = (), Error = ()> + Send>,
}

impl MyReceiver {
    fn say_hi(&self, subject: Subject) {
        println!("Hi {}! It's {}.", subject.name, self.name)
    }
    fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            pending: Box::new(future::ok(())),
        }
    }
}

impl Future for MyReceiver {
    type Item = Self;
    type Error = Self;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Async::Ready(MyReceiver::new(&self.name)))
    }
}

impl Sink for MyReceiver {
    type SinkItem = Subject;
    type SinkError = ();
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.say_hi(item);
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}

#[test]
pub fn try() {
    let svc = MySvc { name: "jOy".into() };

    let task = future::ok(svc)
        .and_then(|s| {
            s.start().and_then(|r| {
                let subject = Subject {
                    name: "Miou".into(),
                };
                let task = stream::once(Ok::<Subject, ()>(subject))
                    .forward(r)
                    .map_err(|_| ())
                    .and_then(|_| Ok(()));
                tokio::spawn(task);
                Ok(())
            })
        })
        .and_then(|_| Ok(()))
        .map_err(|_| ());

    tokio::run(task);
}
