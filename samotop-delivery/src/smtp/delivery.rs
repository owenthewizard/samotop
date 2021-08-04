use crate::dispatch::DispatchMail;
use crate::smtp::{net::Connector, ConnectionReuseParameters, SmtpClient};
use samotop_core::{common::*, mail::*};

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
    /// How to reuse the client:
    ///
    /// * 0 => unlimited resue
    /// * 1 => no reuse
    /// * n => limited to n
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

impl<C: Connector, T: AcceptsDispatch> MailSetup<T> for LmtpDispatch<C>
where
    C: 'static,
    <C as Connector>::Stream: std::fmt::Debug,
{
    fn setup(self, config: &mut T) {
        let transport = self.client.connect_with(self.connector);
        config.add_last_dispatch(DispatchMail::new(transport))
    }
}
