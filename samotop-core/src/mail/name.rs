use crate::builder::{ServerContext, Setup};
use crate::common::*;
use crate::io::{ConnectionInfo, Handler, HandlerService};

/// MailSetup that uses the given service name for a session.
/// It can also attach the instance ID and session ID for better diagnostics.
///
/// Using the default instance or setting name to empty string will reuse the incoming service name already set.
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Name {
    name: String,
    identify_session: bool,
    identify_instance: bool,
    instance_identity: String,
}
impl Name {
    /// Construct a name `MailSetup` to use the given service name.
    /// This name is used in SMTP responses and will be seen in logs.
    /// It is also used to identify a mail transaction.
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            identify_session: false,
            identify_instance: false,
            instance_identity: String::default(),
        }
    }
    /// Switch if instance ID should be included in the service name
    pub fn identify_instance(mut self, enable: bool) -> Self {
        self.identify_instance = enable;
        self
    }
    /// Switch if instance ID should be included in the service name
    pub fn identify_session(mut self, enable: bool) -> Self {
        self.identify_session = enable;
        self
    }
}
impl Handler for Name {
    /// Use a given name as a service name in the session.
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> crate::common::S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        let conn = session
            .store
            .get_or_insert::<ConnectionInfo, _>(|| ConnectionInfo::default());

        let mut name = if self.name.is_empty() {
            std::mem::take(&mut conn.service_name)
        } else {
            self.name.clone()
        };

        if self.identify_instance {
            let instance_id = if self.instance_identity.is_empty() {
                Identify::instance().to_string()
            } else {
                self.instance_identity.clone()
            };
            name = if name.is_empty() {
                instance_id
            } else {
                format!("{}.{}", instance_id, name)
            }
        }

        if self.identify_session {
            let session_id = if conn.id.is_empty() {
                Identify::now().to_string()
            } else {
                std::mem::take(&mut conn.id)
            };
            name = if name.is_empty() {
                session_id
            } else {
                format!("{}.{}", session_id, name)
            };
            conn.id = name.clone();
        };

        conn.service_name = name;

        Box::pin(ready(Ok(())))
    }
}
impl Setup for Name {
    /// Add self as an ESMTP service so it can configure service name for each session
    fn setup(&self, builder: &mut ServerContext) {
        builder.store.add::<HandlerService>(Arc::new(self.clone()))
    }
}
