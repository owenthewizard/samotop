extern crate samotop;
extern crate tokio;

#[test]
#[ignore]
fn use_dead_service() {
    let service = samotop::service::dead::DeadService;
    let server = samotop::model::server::SamotopServer {
        addr: "invalid name:99999999".into(),
        service,
    };
    let task = samotop::server::serve(server);
    tokio::run(task);
}
