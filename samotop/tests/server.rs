extern crate samotop;

use std::sync::Arc;

#[test]
fn use_dummy_service() {
    let _ = samotop::server::TcpServer::default().serve(samotop::io::dummy::DummyService);
}

#[test]
fn use_samotop_server() {
    let _ = samotop::server::TcpServer::default();
}

#[test]
fn builder_builds_task() {
    let parser = samotop::smtp::SmtpParserPeg;
    let mail = samotop::mail::Builder::default()
        .using(samotop::mail::Esmtp.with(parser))
        .into_service();
    let svc = samotop::io::smtp::SmtpService::new(Arc::new(mail));
    let _srv = samotop::server::TcpServer::on("localhost:25").serve(svc);
}
