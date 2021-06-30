//! Provides limited SASL authentication mechanisms

use crate::smtp::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::{Deref, DerefMut};

pub trait Authentication: Send + Sync {
    fn method(&self) -> String;
    fn initialize(&mut self) -> Option<String>;
    fn respond(&mut self, challenge: &str) -> Result<String, Error>;
}
impl<T> Authentication for T
where
    T: Send + Sync,
    T: DerefMut,
    T::Target: Authentication,
{
    fn method(&self) -> String {
        Deref::deref(self).method()
    }
    fn initialize(&mut self) -> Option<String> {
        DerefMut::deref_mut(self).initialize()
    }
    fn respond(&mut self, challenge: &str) -> Result<String, Error> {
        DerefMut::deref_mut(self).respond(challenge)
    }
}

impl Authentication for SimpleAuthentication {
    fn method(&self) -> String {
        format!("{}", self.mechanism)
    }
    fn initialize(&mut self) -> Option<String> {
        match self.mechanism {
            Mechanism::Login => None,
            Mechanism::Plain => Some(format!(
                "\u{0}{}\u{0}{}",
                self.credentials.authentication_identity, self.credentials.secret
            )),
            Mechanism::Xoauth2 => Some(format!(
                "user={}\x01auth=Bearer {}\x01\x01",
                self.credentials.authentication_identity, self.credentials.secret
            )),
        }
    }
    fn respond(&mut self, challenge: &str) -> Result<String, Error> {
        match self.mechanism {
            Mechanism::Plain | Mechanism::Xoauth2 => {
                Err(Error::Client("This mechanism does not expect a challenge"))
            }
            Mechanism::Login => {
                if vec!["User Name", "Username:", "Username"].contains(&challenge) {
                    return Ok(self.credentials.authentication_identity.to_string());
                }

                if vec!["Password", "Password:"].contains(&challenge) {
                    return Ok(self.credentials.secret.to_string());
                }

                Err(Error::Client("Unrecognized challenge"))
            }
        }
    }
}

pub(crate) struct SimpleAuthentication {
    credentials: Credentials,
    mechanism: Mechanism,
}

impl SimpleAuthentication {
    pub fn new(mechanism: Mechanism, credentials: Credentials) -> Self {
        Self {
            credentials,
            mechanism,
        }
    }
}

/// Accepted authentication mechanisms on an encrypted connection
/// Trying LOGIN last as it is deprecated.
pub const DEFAULT_ENCRYPTED_MECHANISMS: &[Mechanism] = &[Mechanism::Plain, Mechanism::Login];

/// Accepted authentication mechanisms on an unencrypted connection
pub const DEFAULT_UNENCRYPTED_MECHANISMS: &[Mechanism] = &[];

/// Convertible to user credentials
pub trait IntoCredentials {
    /// Converts to a `Credentials` struct
    fn into_credentials(self) -> Credentials;
}

impl IntoCredentials for Credentials {
    fn into_credentials(self) -> Credentials {
        self
    }
}

impl<S: Into<String>, T: Into<String>> IntoCredentials for (S, T) {
    fn into_credentials(self) -> Credentials {
        let (username, password) = self;
        Credentials::new(username.into(), password.into())
    }
}

/// Contains user credentials
#[derive(PartialEq, Eq, Clone, Hash, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct Credentials {
    authentication_identity: String,
    secret: String,
}

impl Credentials {
    /// Create a `Credentials` struct from username and password
    pub fn new(username: String, password: String) -> Credentials {
        Credentials {
            authentication_identity: username,
            secret: password,
        }
    }
}

/// Represents authentication mechanisms
#[derive(PartialEq, Eq, Copy, Clone, Hash, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub enum Mechanism {
    /// PLAIN authentication mechanism
    /// RFC 4616: https://tools.ietf.org/html/rfc4616
    Plain,
    /// LOGIN authentication mechanism
    /// Obsolete but needed for some providers (like office365)
    /// https://www.ietf.org/archive/id/draft-murchison-sasl-login-00.txt
    Login,
    /// Non-standard XOAUTH2 mechanism
    /// https://developers.google.com/gmail/imap/xoauth2-protocol
    Xoauth2,
}

impl Display for Mechanism {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Mechanism::Plain => "PLAIN",
                Mechanism::Login => "LOGIN",
                Mechanism::Xoauth2 => "XOAUTH2",
            }
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_plain() {
        let mechanism = Mechanism::Plain;
        let credentials = Credentials::new("username".to_string(), "password".to_string());
        let mut auth = SimpleAuthentication::new(mechanism, credentials);

        assert_eq!(
            auth.initialize().expect("should be some"),
            "\u{0}username\u{0}password"
        );
        assert!(auth.respond("test").is_err());
    }

    #[test]
    fn test_login() {
        let mechanism = Mechanism::Login;
        let credentials = Credentials::new("alice".to_string(), "wonderland".to_string());
        let mut auth = SimpleAuthentication::new(mechanism, credentials);

        assert_eq!(auth.respond("Username").expect("should be some"), "alice");
        assert_eq!(
            auth.respond("Password").expect("should be some"),
            "wonderland"
        );
        assert!(auth.respond("unknown").is_err());
        assert!(auth.respond("").is_err());
    }

    #[test]
    fn test_xoauth2() {
        let mechanism = Mechanism::Xoauth2;
        let credentials = Credentials::new(
            "username".to_string(),
            "vF9dft4qmTc2Nvb3RlckBhdHRhdmlzdGEuY29tCg==".to_string(),
        );
        let mut auth = SimpleAuthentication::new(mechanism, credentials);

        assert_eq!(
            auth.initialize().expect("should be some"),
            "user=username\x01auth=Bearer vF9dft4qmTc2Nvb3RlckBhdHRhdmlzdGEuY29tCg==\x01\x01"
        );
        assert!(auth.respond("test").is_err());
    }
}
