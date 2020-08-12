use tokio;
use tokio::prelude::*;

trait TcpService: Copy {
    type Handler: TcpHandler;
    fn start(&self) -> Self::Handler;
}

trait TcpHandler {
    fn handle(&mut self, some: bool);
}

trait MailService: Copy {
    type Handler: MailHandler;
    fn start(&self) -> Self::Handler;
}

trait MailHandler {
    fn handle(&mut self, some: bool);
}

#[test]
fn services() {
    let mail = MyMail {};
    let tcp = MyTcp { mail };

    tcp.start().handle(true);

    tokio::run(future::ok(tcp).map(|s| {
        let fut = future::lazy(move || Ok(s.start().handle(true)));
        tokio::spawn(fut);
        ()
    }));
}

#[derive(Copy, Clone)]
struct MyTcp<M> {
    mail: M,
}

impl<M> TcpService for MyTcp<M>
where
    M: MailService + Copy,
{
    type Handler = MyTcpHandler<M>;
    fn start(&self) -> Self::Handler {
        MyTcpHandler { mail: self.mail }
    }
}

struct MyTcpHandler<M> {
    mail: M,
}

impl<M> TcpHandler for MyTcpHandler<M>
where
    M: MailService + Copy,
{
    fn handle(&mut self, some: bool) {
        self.mail.start().handle(some)
    }
}

#[derive(Copy, Clone)]
struct MyMail {}

impl MailService for MyMail {
    type Handler = MyMailHandler;
    fn start(&self) -> Self::Handler {
        MyMailHandler {}
    }
}

struct MyMailHandler {}

impl MailHandler for MyMailHandler {
    fn handle(&mut self, some: bool) {
        println!("got {}", some)
    }
}
