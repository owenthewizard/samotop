extern crate samotop;
extern crate tokio;

use tokio::prelude::*;

#[test]
pub fn use_dead_service() {
    let service = samotop::service::dead::DeadService;
    let server = samotop::model::next::SamotopServer {
        addr: "localhost:1".into(),
        service,
    };
    let task = samotop::server::next::serve(server);
    tokio::run(task);
}
