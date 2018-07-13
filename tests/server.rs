extern crate samotop;

#[test]
fn use_dead_service() {
    let _ = samotop::builder()
        .with(samotop::service::tcp::DeadService)
        .build_task();
}

#[test]
fn use_samotop_service() {
    let _ = samotop::builder().build_task();
}

#[test]
fn builder_builds_task() {
    let mail = samotop::service::mail::ConsoleMail::new("Aloha");
    let sess = samotop::service::session::StatefulSessionService::new(mail);
    let svc = samotop::service::tcp::SamotopService::new(sess, Default::default());
    let _task = samotop::server::SamotopBuilder::new("localhost:25", svc).build_task();
}
