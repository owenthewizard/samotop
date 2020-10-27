extern crate samotop;

#[test]
fn use_dummy_service() {
    let _ = samotop::server::Server::default().serve(samotop::service::tcp::dummy::DummyTcpService);
}

#[test]
fn use_samotop_server() {
    let _ = samotop::server::Server::default();
}

#[test]
fn builder_builds_task() {
    let mail = samotop::service::mail::default::DefaultMailService::default();
    let parser = samotop::service::parser::SmtpParser;
    let svc = samotop::service::tcp::smtp::SmtpService::new(mail, parser);
    let svc = samotop::service::tcp::tls::TlsEnabled::disabled(svc);
    let _srv = samotop::server::Server::on("localhost:25").serve(svc);
}
