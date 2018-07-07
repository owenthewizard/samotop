extern crate samotop;

#[test]
fn use_dead_service() {
    let _ = samotop::builder()
        .with(samotop::service::dead::DeadService)
        .as_task();
}

#[test]
fn use_samotop_service() {
    let _ = samotop::builder()
        .with(samotop::service::samotop::SamotopService::new("name"))
        .as_task();
}
