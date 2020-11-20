use super::dispatch::DispatchMail;
use crate::smtp::{net::Connector, ConnectionReuseParameters, SmtpClient};
use samotop_model::{common::*, mail::*};

#[derive(Debug)]
pub struct LmtpDispatch<C: Connector> {
    pub client: SmtpClient,
    pub connector: C,
}

impl<C: Connector> LmtpDispatch<C> {
    pub fn new(address: String, connector: C) -> Result<Self> {
        let variant = LmtpDispatch {
            client: SmtpClient::new(&address)?.lmtp(true),
            connector,
        };
        Ok(variant)
    }
    pub fn reuse(mut self, lifetimes: u16) -> Self {
        self.client = match lifetimes {
            0 => self
                .client
                .connection_reuse(ConnectionReuseParameters::ReuseUnlimited),
            1 => self
                .client
                .connection_reuse(ConnectionReuseParameters::NoReuse),
            n => self
                .client
                .connection_reuse(ConnectionReuseParameters::ReuseLimited(n - 1)),
        };
        self
    }
}

impl<C: Connector> MailSetup for LmtpDispatch<C>
where
    C: 'static,
    <C as Connector>::Stream: std::fmt::Debug,
{
    fn setup(self, builder: &mut Builder) {
        let transport = self.client.connect_with(self.connector);
        builder
            .dispatch
            .insert(0, Box::new(DispatchMail::new(transport)))
    }
}
