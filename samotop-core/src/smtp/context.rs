use crate::{
    common::{Arc, Dummy},
    io::ConnectionInfo,
    mail::MailService,
    smtp::SmtpSession,
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

#[derive(Debug, Default)]
pub struct SmtpContext {
    /// Implementation-specific value store
    store: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    pub session: SmtpSession,
}

impl SmtpContext {
    pub fn new<Svc>(service: Svc, connection: ConnectionInfo) -> Self
    where
        Svc: MailService + Sync + Send + 'static,
    {
        let mut me = SmtpContext {
            session: SmtpSession::new(connection),
            ..Default::default()
        };
        me.set_service(service);
        me
    }
}

impl SmtpContext {
    pub fn emtpy() -> Self {
        Self::default()
    }
    pub fn get<T: Sync + Send + 'static>(&self) -> Option<&T> {
        self.store
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref())
    }
    pub fn get_mut<T: Sync + Send + 'static>(&mut self) -> Option<&mut T> {
        self.store
            .get_mut(&TypeId::of::<T>())
            .and_then(|v| v.downcast_mut())
    }
    pub fn get_or_insert<T: Sync + Send + 'static, F>(&mut self, insert: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        let id = TypeId::of::<T>();
        self.store
            .entry(id)
            .or_insert_with(|| Box::new(insert()))
            .downcast_mut::<T>()
            .expect("stored type must match")
    }
    pub fn set<T: Sync + Send + 'static>(&mut self, value: T) {
        let id = TypeId::of::<T>();
        self.store.insert(id, Box::new(value));
    }
    pub fn service(&self) -> impl MailService {
        self.get::<Arc<dyn MailService + Send + Sync + 'static>>()
            .cloned()
            .unwrap_or_else(|| Arc::new(Dummy) as Arc<dyn MailService + Send + Sync>)
    }
    pub(crate) fn set_service(&mut self, service: impl MailService + Send + Sync + 'static) {
        let service = Arc::new(service) as Arc<dyn MailService + Send + Sync + 'static>;
        self.set(service);
    }
}

/// Represents the instructions for the client side of the stream.
#[derive(Clone, Eq, PartialEq)]
pub enum DriverControl {
    /// Write an SMTP response
    Response(Vec<u8>),
    /// Start TLS encryption
    StartTls,
    /// Shut the stream down
    Shutdown,
}

impl std::fmt::Debug for DriverControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        enum TextOrBytes<'a> {
            T(&'a str),
            B(&'a [u8]),
        }
        fn tb(inp: &[u8]) -> TextOrBytes {
            if let Ok(text) = std::str::from_utf8(inp) {
                TextOrBytes::T(text)
            } else {
                TextOrBytes::B(inp)
            }
        }
        match self {
            DriverControl::Response(r) => f.debug_tuple("Response").field(&tb(r)).finish(),
            DriverControl::StartTls => f.debug_tuple("StartTls").finish(),
            DriverControl::Shutdown => f.debug_tuple("Shutdown").finish(),
        }
    }
}

#[cfg(test)]
mod store_tests {
    use std::time::SystemTime;

    use super::*;
    use crate::mail::Builder;
    use regex::Regex;

    #[test]
    pub fn same_service() {
        let mut sut = SmtpContext::default();
        let svc = Box::new(Builder.build());
        let dump0 = format!("{:#?}", svc);
        sut.set_service(Dummy);
        sut.set_service(svc);

        let dump1 = format!("{:#?}", sut.service());
        assert_eq!(dump1, dump0);

        let dump = Regex::new("[0-9]+")
            .expect("regex")
            .replace_all(&dump0, "--redaced--");
        insta::assert_display_snapshot!(dump, @r###"
        Service {
            session: EsmtpBunch {
                id: "--redaced--",
                items: [],
            },
            guard: GuardBunch {
                id: "--redaced--",
                items: [],
            },
            dispatch: DispatchBunch {
                id: "--redaced--",
                items: [],
            },
            driver: SmtpDriver,
            interpret: Interpretter(--redaced--),
        }
        "###);
    }

    #[test]
    pub fn set_one_service() {
        let mut sut = SmtpContext::default();
        sut.set_service(Box::new(Dummy));
        sut.set_service(Builder.build());

        sut.session.connection.id = "--redacted--".into();
        sut.session.connection.established = SystemTime::UNIX_EPOCH;

        let dump = format!("{:#?}", sut);

        insta::assert_display_snapshot!(dump, @r###"
        SmtpContext {
            store: {
                TypeId {
                    t: 15396228761292846990,
                }: Any { .. },
            },
            session: SmtpSession {
                connection: ConnectionInfo {
                    id: "--redacted--",
                    local_addr: "",
                    peer_addr: "",
                    established: SystemTime {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                },
                extensions: ExtensionSet {
                    map: {},
                },
                service_name: "",
                peer_name: None,
                output: [],
                input: [],
                mode: None,
                transaction: Transaction {
                    id: "",
                    mail: None,
                    rcpts: [],
                    extra_headers: "",
                    sink: "*",
                },
            },
        }
        "###);
    }
}
