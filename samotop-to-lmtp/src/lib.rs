use async_smtp::{ClientSecurity, Envelope, SendableEmail, SmtpClient, Transport};
use samotop_core::common::*;
use samotop_core::service::mail::composite::*;
use samotop_core::service::mail::*;
use std::marker::PhantomData;

struct Config<Variant> {
    phantom: PhantomData<Variant>,
}

pub mod variants {
    pub struct Delivery;
}

impl Config<variants::Delivery> {
    fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<NS: NamedService, ES: EsmtpService, GS: MailGuard, QS: MailQueue> MailSetup<NS, ES, GS, QS>
    for Config<variants::Delivery>
{
    type Output = CompositeMailService<NS, ES, GS, LmtpMail<variants::Delivery>>;
    fn setup(self, named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        (named, extend, guard, LmtpMail::new(self)).into()
    }
}

struct LmtpMail<Variant> {
    config: Config<Variant>,
}

impl<Any> LmtpMail<Any> {
    fn new(config: Config<Any>) -> Self {
        Self { config }
    }
}

impl<Any> MailQueue for LmtpMail<Any> {
    type Mail = Vec<u8>;
    type MailFuture = future::Ready<Option<Self::Mail>>;
    fn mail(&self, mail: samotop_core::model::mail::Envelope) -> Self::MailFuture {
        unimplemented!()
    }
    fn new_id(&self) -> String {
        unimplemented!()
    }
}

async fn smtp_transport_simple() -> Result<()> {
    let email = SendableEmail::new(
        Envelope::new(
            Some("user@localhost".parse().unwrap()),
            vec!["root@localhost".parse().unwrap()],
        )?,
        "id",
        "Hello world",
    );

    // Create a client
    let mut smtp = SmtpClient::with_security("127.0.0.1:2525", ClientSecurity::None)
        .await?
        .into_transport();

    // Connect and send the email.
    smtp.send(email).await?;

    Ok(())
}
