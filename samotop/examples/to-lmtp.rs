/*!
Example of delivering to LMTP over unix socket
 */

use async_std::task;
use regex::Regex;
use samotop::service::mail::default::DefaultMailService;
use samotop::service::mail::lmtp::Config as LmtpConfig;
use samotop::service::mail::MailServiceBuilder;
use samotop::service::parser::SmtpParser;
use samotop::service::tcp::{smtp::SmtpService, tls::TlsEnabled};
use samotop::service::{client::UnixConnector, mail::mapper::Config};
use samotop::{server::Server, service::client::tls::NoTls};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let rcpt_map = Config::new(vec![
        (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()), // use domain as a user name (all domain basket) anyone@example.org => example.org@localhost
        (Regex::new("[^@a-zA-Z0-9]+")?, "-".to_owned()), // sanitize the user name example.org@localhost => example-org@localhost
    ]);
    let lmtp_connector: UnixConnector<NoTls> = UnixConnector::default();
    let mail_service = DefaultMailService::new("test-samotop".to_owned())
        .using(
            LmtpConfig::lmtp_dispatch("/var/run/dovecot/lmtp".to_owned(), lmtp_connector)?.reuse(0),
        )
        .using(rcpt_map);
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);

    Server::on("localhost:2525").serve(tls_smtp_service).await
}
