use std::io;

use crate::common::*;
use crate::mail::MailService;
use crate::mail::*;
use crate::session::stateful::SessionHandler;
use crate::smtp::*;

#[derive(Clone)]
pub struct BasicSessionHandler<S> {
    service: S,
}

impl<S> From<S> for BasicSessionHandler<S>
where
    S: MailService,
{
    fn from(service: S) -> Self {
        Self { service }
    }
}

impl<S: MailService> SessionHandler for BasicSessionHandler<S> {
    type Data = SmtpStateBase;
    fn pop(&self, data: &mut Self::Data) -> Option<WriteControl> {
        data.pop()
    }
    fn handle(&self, data: Self::Data, control: ReadControl) -> S3Fut<Self::Data> {
        match control {
            ReadControl::PeerConnected(conn) => self.handle_conn(data, conn),
            ReadControl::PeerShutdown => self.handle_shutdown(data),
            ReadControl::Raw(_) => self.handle_raw(data),
            ReadControl::Command(cmd, _) => self.handle_cmd(data, cmd),
            ReadControl::MailDataChunk(bytes) => self.handle_data_chunk(data, bytes),
            ReadControl::EndOfMailData(_) => self.handle_data_end(data),
            ReadControl::Empty(_) => Box::pin(ready(data)),
            ReadControl::EscapeDot(_) => Box::pin(ready(data)),
        }
    }
}

impl<S> BasicSessionHandler<S> {
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S: MailService> BasicSessionHandler<S> {
    pub fn handle_cmd(&self, data: SmtpStateBase, cmd: SmtpCommand) -> S3Fut<SmtpStateBase> {
        use SmtpCommand::*;
        match cmd {
            Helo(from) => self.cmd_helo(data, from),
            Mail(mail) => self.cmd_mail(data, mail),
            Rcpt(path) => self.cmd_rcpt(data, path),
            Data => self.cmd_data(data),
            Quit => self.cmd_quit(data),
            Rset => self.cmd_rset(data),
            Noop(_) => self.cmd_noop(data),
            StartTls => self.cmd_starttls(data),
            Expn(_) => self.cmd_unknown(data),
            Vrfy(_) => self.cmd_unknown(data),
            Help(_) => self.cmd_unknown(data),
            Turn => self.cmd_unknown(data),
            Other(_, _) => self.cmd_unknown(data),
        }
    }
    pub fn handle_data_chunk(
        &self,
        mut data: SmtpStateBase,
        bytes: Vec<u8>,
    ) -> S3Fut<SmtpStateBase> {
        if data.transaction().sink.is_none() {
            // CheckMe: silence. handle_data_end should respond with error.
            return Box::pin(ready(data));
        }
        let mut sink = data
            .transaction_mut()
            .sink
            .take()
            .expect("Checked presence above");
        let mailid = data.transaction().id.clone();
        let fut = async move {
            match sink.write_all(&bytes[..]).await {
                Ok(()) => {
                    data.transaction_mut().sink = Some(sink);
                    data
                }
                Err(e) => {
                    warn!("Failed to write mail data for {} - {}", mailid, e);
                    data.reset();
                    // CheckMe: following this reset, we are not sending any response yet. handle_data_end should do that.
                    data
                }
            }
        };
        Box::pin(fut)
    }
    pub fn handle_data_end(&self, mut data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        if data.transaction().sink.is_none() {
            // CheckMe: silence. handle_data_end should respond with error.
            return Box::pin(ready(data));
        }
        let mut sink = data
            .transaction_mut()
            .sink
            .take()
            .expect("Checked presence above");
        let mailid = data.transaction().id.clone();
        let fut = async move {
            if match sink.close().await {
                Ok(()) => true,
                Err(e) if e.kind() == io::ErrorKind::NotConnected => true,
                Err(e) => {
                    warn!("Failed to close mail {}: {}", mailid, e);
                    false
                }
            } {
                data.say_mail_queued(mailid.as_str());
            } else {
                data.say_mail_queue_failed_temporarily();
            }
            data.reset();
            data
        };
        Box::pin(fut)
    }
    pub fn handle_raw(&self, mut data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        data.say_reply(SmtpReply::CommandSyntaxFailure);
        Box::pin(ready(data))
    }
    pub fn handle_conn(
        &self,
        mut data: SmtpStateBase,
        mut sess: SessionInfo,
    ) -> S3Fut<SmtpStateBase> {
        self.service.prepare_session(&mut sess);
        let name = sess.service_name.to_owned();
        data.reset();
        *data.session_mut() = sess;
        data.say_service_ready(name);
        Box::pin(ready(data))
    }
    pub fn handle_shutdown(&self, mut data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        data.reset();
        Box::pin(ready(data))
    }

    fn cmd_unknown(&self, mut data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        data.say_not_implemented();
        Box::pin(ready(data))
    }
    fn cmd_helo(&self, data: SmtpStateBase, helo: SmtpHelo) -> S3Fut<SmtpStateBase> {
        helo.apply(data)
    }
    fn cmd_quit(&self, data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        SmtpQuit.apply(data)
    }
    fn cmd_mail(&self, data: SmtpStateBase, mail: SmtpMail) -> S3Fut<SmtpStateBase> {
        mail.apply(data)
    }
    fn cmd_rcpt(&self, data: SmtpStateBase, rcpt: SmtpPath) -> S3Fut<SmtpStateBase> {
        SmtpRcpt::from(rcpt).apply(data)
    }
    fn cmd_data(&self, data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        SmtpData.apply(data)
    }
    fn cmd_rset(&self, data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        SmtpRset.apply(data)
    }
    fn cmd_noop(&self, data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        SmtpNoop.apply(data)
    }
    fn cmd_starttls(&self, data: SmtpStateBase) -> S3Fut<SmtpStateBase> {
        StartTls.apply(data)
    }
}
