extern crate samotop;

#[test]
fn use_dummy_service() {
    let _ = samotop::server::Server::default().serve(samotop::io::dummy::DummyTcpService);
}

#[test]
fn use_samotop_server() {
    let _ = samotop::server::Server::default();
}

#[test]
fn builder_builds_task() {
    let mail = samotop::mail::DefaultMailService::default();
    let parser = samotop::parser::SmtpParser;
    let svc = samotop::io::smtp::SmtpService::new(mail, parser);
    let svc = samotop::io::tls::TlsEnabled::disabled(svc);
    let _srv = samotop::server::Server::on("localhost:25").serve(svc);
}
