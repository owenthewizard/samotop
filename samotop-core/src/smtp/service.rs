use crate::{
    common::{Result, S1Fut},
    io::{tls::MayBeTls, ConnectionInfo, IoService},
    mail::Service,
    smtp::{EsmtpService, SmtpState},
};

impl IoService for Service {
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S1Fut<'static, Result<()>> {
        let service = self.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);
            let mut io = io?;
            let mut state = SmtpState::new(service.clone());
            state.session.connection = connection;

            service.prepare_session(&mut io, &mut state).await;

            // fetch and apply commands
            state.service.get_driver(&mut io).drive(&mut state).await?;

            Ok(())
        })
    }
}
