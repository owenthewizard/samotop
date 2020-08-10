//! Reference implementation of a mail service
//! simply delivering mail to single directory.
use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::model::Error;
use crate::service::mail::*;
use async_std::fs::{create_dir_all, rename, File};
use async_std::path::Path;
use futures::future::TryFutureExt;

#[derive(Clone)]
pub struct SimpleDirMail<N, D> {
    name: N,
    dir: D,
}

impl<N, D> SimpleDirMail<N, D>
where
    N: AsRef<str>,
    D: AsRef<Path>,
{
    pub fn new(name: N, dir: D) -> Self {
        Self { name, dir }
    }
}

impl<N, D> NamedService for SimpleDirMail<N, D>
where
    N: AsRef<str>,
{
    fn name(&self) -> &str {
        self.name.as_ref()
    }
}

impl<N, D> EsmtpService for SimpleDirMail<N, D> {
    fn extend(&self, connection: &mut Connection) {
        connection.enable(SmtpExtension::EightBitMime);
    }
}

impl<N, D> MailGuard for SimpleDirMail<N, D> {
    type Future = futures::future::Ready<AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        println!("Accepting recipient {:?}", request);
        future::ready(AcceptRecipientResult::Accepted(request.rcpt))
    }
}

impl<N, D> MailQueue for SimpleDirMail<N, D>
where
    D: AsRef<Path>,
{
    type Mail = MailFile;
    type MailFuture = CreateMailFile;

    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        CreateMailFile::new(&self.dir, envelope)
    }
}

#[pin_project]
pub struct CreateMailFile {
    stage2: Option<(
        BytesMut,
        String,
        Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>,
    )>,
    file: Pin<Box<dyn Future<Output = std::io::Result<File>> + Send>>,
}

impl CreateMailFile {
    pub fn new<D: AsRef<Path>>(dir: D, envelope: Envelope) -> Self {
        let mut headers = BytesMut::new();
        headers.extend(format!("X-SamotopHelo: {:?}\r\n", envelope.helo).bytes());
        headers.extend(format!("X-SamotopPeer: {:?}\r\n", envelope.peer).bytes());
        headers.extend(format!("X-SamotopMailFrom: {:?}\r\n", envelope.mail).bytes());
        headers.extend(format!("X-SamotopRcptTo: {:?}\r\n", envelope.rcpts).bytes());

        let target_dir = dir.as_ref().join("new");
        let tmp_dir = dir.as_ref().join("tmp");
        let target_file = target_dir.join(envelope.id.as_str());
        let tmp_file = tmp_dir.join(envelope.id.as_str());
        let target = Box::pin(rename(tmp_file.clone(), target_file.clone()));
        let file = Box::pin(
            ensure_dir(tmp_dir)
                .and_then(move |_| ensure_dir(target_dir))
                .and_then(move |_| File::create(tmp_file)),
        );

        Self {
            stage2: Some((headers, envelope.id, target)),
            file,
        }
    }
}

async fn ensure_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    if !dir.as_ref().exists().await {
        create_dir_all(dir).await
    } else {
        Ok(())
    }
}

impl Future for CreateMailFile {
    type Output = Option<MailFile>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<MailFile>> {
        match ready!(Pin::new(&mut self.file).poll(cx)) {
            Ok(file) => {
                if let Some((buffer, id, target)) = self.stage2.take() {
                    Poll::Ready(Some(MailFile {
                        id,
                        file,
                        buffer,
                        target,
                    }))
                } else {
                    error!("No buffer/id. Perhaps the future has been polled after Poll::Ready");
                    Poll::Ready(None)
                }
            }
            Err(e) => {
                error!("Could not create mail file: {:?}", e);
                Poll::Ready(None)
            }
        }
    }
}

#[pin_project(project=MailFileProj)]
pub struct MailFile {
    id: String,
    file: File,
    buffer: BytesMut,
    target: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>,
}

impl Sink<Bytes> for MailFile {
    type Error = Error;
    fn start_send(mut self: Pin<&mut Self>, bytes: Bytes) -> Result<()> {
        println!("Mail data for {}: {} bytes", self.id, bytes.len());
        self.buffer.extend(bytes);
        Ok(())
    }
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let MailFileProj { file, buffer, .. } = self.project();
        let mut pending = &buffer[..];
        let mut file = Pin::new(file);
        trace!("Writing mail data: {} bytes", pending.len());
        while let Poll::Ready(len) = file.as_mut().poll_write(cx, pending)? {
            trace!("Wrote mail data: {} bytes", len);
            if len == 0 {
                break;
            }
            pending = &pending[len..];
        }
        // remove written bytes from the buffer
        let written = buffer.len() - pending.len();
        drop(buffer.split_to(written));
        trace!("Remaining {} bytes", buffer.len());
        if buffer.len() == 0 {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_ready(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        ready!(self.as_mut().poll_flush(cx))?;
        let MailFileProj { target, .. } = self.project();
        ready!(target.as_mut().poll(cx))?;
        Poll::Ready(Ok(()))
    }
}

// impl Mail for MailFile {
//     fn queue_id(&self) -> &str {
//         self.id.as_ref()
//     }
// }
