mod tcp;
#[cfg(unix)]
mod unix;
pub use self::tcp::*;
#[cfg(unix)]
pub use self::unix::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_dummy_service() {
        let _ = TcpServer::default().serve(crate::io::dummy::DummyService);
    }

    #[test]
    fn use_samotop_server() {
        let _ = TcpServer::default();
    }

    #[test]
    fn builder_builds_task() {
        let mail = crate::mail::Builder::default().build();
        let _srv = crate::server::TcpServer::on("localhost:25").serve(mail);
    }
}
