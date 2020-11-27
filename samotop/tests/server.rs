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
    let parser = samotop::parser::SmtpParser;
    let mail = Arc::new(samotop::mail::Builder::default().using(parser));
    let svc = samotop::io::smtp::SmtpService::new(mail);
    let svc = samotop::io::tls::TlsEnabled::disabled(svc);
    let _srv = samotop::server::TcpServer::on("localhost:25").serve(svc);
}
