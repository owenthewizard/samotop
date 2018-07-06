use tokio;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;

struct Subject {
    name: String,
}

trait Svc {
    type Future;
    fn handle(&self, subject: Subject) -> Self::Future;
}

struct SizedSvc {
    name: String,
}
impl Svc for SizedSvc {
    type Future = FutureResult<(), ()>;
    fn handle(&self, subject: Subject) -> Self::Future {
        future::ok(println!(
            "MySvc. Hi {}! My name is {}.",
            subject.name, self.name
        ))
    }
}

#[derive(Clone)]
struct BoxSvc1 {
    name: String,
}
impl Svc for BoxSvc1 {
    type Future = Box<Future<Item = (), Error = ()>>;
    fn handle(&self, subject: Subject) -> Self::Future {
        Box::new(
            future::ok((self.name.clone(), subject))
                .and_then(|(n, s)| Ok(println!("BoxSvc1. Hi {}! My name is {}.", s.name, n))),
        )
    }
}

#[derive(Clone)]
struct BoxSvc2 {
    name: String,
}
impl Svc for BoxSvc2 {
    type Future = Box<Future<Item = (), Error = ()>>;
    fn handle(&self, subject: Subject) -> Self::Future {
        Box::new(future::ok(println!(
            "Hi {}! My name is {}.",
            subject.name, self.name
        )))
    }
}

struct Server<S> {
    service: S,
}

impl<S> Server<S>
where
    S: Svc,
    S::Future: Future<Item = (), Error = ()> + Send + 'static,
{
    pub fn run(&self) -> impl Future<Item = (), Error = ()> {
        let subject = Subject {
            name: "Gandalf".into(),
        };
        let spawn = tokio::spawn(self.service.handle(subject));
        spawn.into_future()
    }
}

#[test]
fn server_works() {
    // this works, but only because we know exactly te future type
    // and as such it's not flexible at all
    let srv1 = Server {
        service: SizedSvc {
            name: "Zorg".into(),
        },
    };
    let _srv2 = Server {
        service: BoxSvc1 {
            name: "Darth".into(),
        },
    };
    let _srv3 = Server {
        service: BoxSvc2 {
            name: "Shadow".into(),
        },
    };
    tokio::run(future::ok(srv1).and_then(|s| s.run()));
    //tokio::run(future::ok(srv2).and_then(|s| s.run()));
    //tokio::run(future::ok(srv3).and_then(|s| s.run()));
}
