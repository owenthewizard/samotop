use crate::smtp::authentication::{
    Authentication, Credentials, Mechanism, SimpleAuthentication, DEFAULT_ENCRYPTED_MECHANISMS,
    DEFAULT_UNENCRYPTED_MECHANISMS,
};
use crate::smtp::error::Error;
use crate::smtp::extension::ClientId;
use crate::smtp::extension::ServerInfo;
use crate::smtp::net::{ConnectionConfiguration, Connector, DefaultConnector};
use crate::smtp::response::Response;
use crate::smtp::stream::SmtpDataStream;
use crate::smtp::SmtpTransport;
use crate::{Envelope, Transport};
use async_std::io::Read;
use std::time::Duration;

// Registered port numbers:
// https://www.iana.
// org/assignments/service-names-port-numbers/service-names-port-numbers.xhtml

/// Default smtp port
pub const SMTP_PORT: u16 = 25;
/// Default submission port
pub const SUBMISSION_PORT: u16 = 587;
/// Default submission over TLS port
pub const SUBMISSIONS_PORT: u16 = 465;

/// How to apply TLS to a client connection
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClientSecurity {
    /// Insecure connection only (for testing purposes)
    None,
    /// Start with insecure connection and use `STARTTLS` when available
    Opportunistic,
    /// Start with insecure connection and require `STARTTLS`
    Required,
    /// Use TLS wrapped connection
    Wrapper,
}

/// Configures connection reuse behavior
#[derive(Clone, Debug, Copy)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum ConnectionReuseParameters {
    /// Unlimited connection reuse
    ReuseUnlimited,
    /// Maximum number of connection reuse
    ReuseLimited(u16),
    /// Disable connection reuse, close connection after each transaction
    NoReuse,
}

/// Contains client configuration
#[derive(Debug)]
#[allow(missing_debug_implementations)]
pub struct SmtpClient {
    /// Enable connection reuse
    pub(crate) connection_reuse: ConnectionReuseParameters,
    /// Name sent during EHLO
    pub(crate) hello_name: ClientId,
    /// Credentials
    pub(crate) credentials: Option<Credentials>,
    /// Socket we are connecting to
    pub(crate) server_addr: String,
    /// TLS security configuration
    pub(crate) security: ClientSecurity,
    // FIXME: Either we transcode based on available server options
    //        Or we have to let the user handle unsupported encoding
    //        Simple on/off option would lead to malformed mail data
    // /// Enable UTF8 mailboxes in envelope or headers
    // pub(crate) smtp_utf8: bool,
    /// Optional enforced authentication mechanism
    pub(crate) authentication_mechanism: Option<Vec<Mechanism>>,
    /// Force use of the set authentication mechanism even if server does not report to support it
    pub(crate) force_set_auth: bool,
    /// Define network timeout
    /// It can be changed later for specific needs (like a different timeout for each SMTP command)
    pub(crate) timeout: Option<Duration>,
}

/// Builder for the SMTP `SmtpTransport`
impl SmtpClient {
    /// Creates a new SMTP client
    ///
    /// Defaults are:
    ///
    /// * No connection reuse
    /// * No authentication
    /// * No SMTPUTF8 support
    /// * A 60 seconds timeout for smtp commands
    ///
    /// Consider using [`SmtpClient::new_simple`] instead, if possible.
    pub fn with_security<A: ToString>(
        address: A,
        security: ClientSecurity,
    ) -> Result<SmtpClient, Error> {
        let mut me = SmtpClient::new(address)?;
        me.security = security;
        Ok(me)
    }

    /// Simple and secure transport, should be used when possible.
    /// Creates an encrypted transport over submissions port, using the provided domain
    /// to validate TLS certificates.
    pub fn new<A: ToString>(address: A) -> Result<SmtpClient, Error> {
        Ok(SmtpClient {
            server_addr: address.to_string(),
            security: ClientSecurity::Opportunistic,
            //smtp_utf8: false,
            credentials: None,
            connection_reuse: ConnectionReuseParameters::NoReuse,
            hello_name: Default::default(),
            authentication_mechanism: None,
            force_set_auth: false,
            timeout: Some(Duration::new(60, 0)),
        })
    }

    /// Creates a new local SMTP client to port 25
    pub fn new_unencrypted_localhost() -> Result<SmtpClient, Error> {
        let mut me = SmtpClient::new("localhost:25")?;
        me.security = ClientSecurity::None;
        Ok(me)
    }

    // FIXME: see field
    // /// Enable SMTPUTF8 if the server supports it
    // pub fn smtp_utf8(mut self, enabled: bool) -> SmtpClient {
    //     self.smtp_utf8 = enabled;
    //     self
    // }

    /// Set the name used during EHLO
    pub fn hello_name(mut self, name: ClientId) -> SmtpClient {
        self.hello_name = name;
        self
    }

    /// Enable connection reuse
    pub fn connection_reuse(mut self, parameters: ConnectionReuseParameters) -> SmtpClient {
        self.connection_reuse = parameters;
        self
    }

    /// Set the client credentials
    pub fn credentials<S: Into<Credentials>>(mut self, credentials: S) -> SmtpClient {
        self.credentials = Some(credentials.into());
        self
    }

    /// Set the authentication mechanism to use
    pub fn authentication_mechanism(mut self, mechanism: Vec<Mechanism>) -> SmtpClient {
        self.authentication_mechanism = Some(mechanism);
        self
    }

    /// Set if the set authentication mechanism should be force
    pub fn force_set_auth(mut self, force: bool) -> SmtpClient {
        self.force_set_auth = force;
        self
    }

    /// Set the timeout duration
    pub fn timeout(mut self, timeout: Option<Duration>) -> SmtpClient {
        self.timeout = timeout;
        self
    }

    /// Build the SMTP client transport
    ///
    /// The transport connects on first use and can be reused if configured so
    /// to send multiple e-mails. Default TCP/TLS connectors are used.
    pub fn connect(self) -> SmtpTransport<Self, DefaultConnector> {
        self.connect_with(DefaultConnector::default())
    }

    /// Build the SMTP client transport
    ///
    /// The transport connects on first use and can be reused if configured so
    /// to send multiple e-mails. Using the provided connector you can control
    /// how the address is resolved, connection established, TLS negotiated.
    pub fn connect_with<C: Connector>(self, connector: C) -> SmtpTransport<Self, C> {
        SmtpTransport::new(self, connector)
    }

    /// Connect to the server and send one mail
    pub async fn connect_and_send<R>(
        self,
        envelope: Envelope,
        message: R,
    ) -> Result<Response, Error>
    where
        R: Read + Unpin + Send + Sync,
    {
        self.connect().send(envelope, message).await
    }

    /// Connect to the server and send one mail, returning data stream to write body into
    pub async fn connect_and_send_stream(
        self,
        envelope: Envelope,
    ) -> Result<SmtpDataStream<<DefaultConnector as Connector>::Stream>, Error> {
        self.connect().send_stream(envelope).await
    }
}

impl ConnectionConfiguration for SmtpClient {
    fn address(&self) -> String {
        self.server_addr.clone()
    }
    fn timeout(&self) -> Duration {
        self.timeout.clone().unwrap_or_default()
    }
    fn security(&self) -> ClientSecurity {
        self.security
    }
    fn hello_name(&self) -> ClientId {
        self.hello_name.clone()
    }
    fn max_reuse_count(&self) -> u16 {
        match self.connection_reuse {
            ConnectionReuseParameters::ReuseUnlimited => u16::MAX,
            ConnectionReuseParameters::ReuseLimited(n) => n,
            ConnectionReuseParameters::NoReuse => 0,
        }
    }
    fn get_authentication(
        &self,
        server_info: &ServerInfo,
        encrypted: bool,
    ) -> Option<Box<dyn Authentication>> {
        let credentials = self.credentials.clone()?;

        let accepted_mechanisms = match self.authentication_mechanism {
            Some(ref mechanisms) => mechanisms,
            None => {
                if encrypted {
                    DEFAULT_ENCRYPTED_MECHANISMS
                } else {
                    DEFAULT_UNENCRYPTED_MECHANISMS
                }
            }
        };

        if let Some(mechanism) = accepted_mechanisms
            .iter()
            .find(|mechanism| server_info.supports_auth_mechanism(**mechanism))
        {
            // Use the first mechanism that agrees with the server
            Some(Box::new(SimpleAuthentication::new(*mechanism, credentials)))
        } else if let (true, Some(mechanism)) = (self.force_set_auth, accepted_mechanisms.get(0)) {
            // We did not agree with the server, but we'll try to force it
            Some(Box::new(SimpleAuthentication::new(*mechanism, credentials)))
        } else {
            None
        }
    }
}
