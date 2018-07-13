use model::controll::*;
use native_tls::*;
use std;
use std::str;
use tokio::io;
use tokio::prelude::*;

pub fn tls_capable<IO>(io: IO, config: TlsWorker) -> TlsCapable<IO> {
    TlsCapable {
        io: Some(Tls::Ready(Usable::Plain(io))),
        config,
    }
}

pub struct TlsCapable<IO> {
    io: Option<Tls<IO>>,
    config: TlsWorker,
}

enum Usable<IO> {
    Plain(IO),
    Encrypted(TlsStream<IO>),
}

enum Tls<IO> {
    Ready(Usable<IO>),
    Negotiating(MidHandshakeTlsStream<IO>),
    Failed(io::Error),
}

impl<IO> std::fmt::Debug for Tls<IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Tls::Failed(ref e) => write!(f, "Failed({:?})", e),
            Tls::Negotiating(_) => write!(f, "Negotiating(...)"),
            Tls::Ready(Usable::Encrypted(_)) => write!(f, "Ready(Encrypted(...))"),
            Tls::Ready(Usable::Plain(_)) => write!(f, "Ready(Plain(...))"),
        }
    }
}

impl<IO> TlsCapable<IO> {
    /// fix the io if necessary and return the usable io or err
    fn check<'a>(&'a mut self, upgrade: bool) -> io::Result<&'a mut Usable<IO>>
    where
        IO: Read + Write,
    {
        trace!("Fixing!");
        self.fix(upgrade);
        trace!("Fixed {:?}", self.io);
        match self.io {
            None => Err(Self::err_no_io()),
            Some(ref mut tls) => match tls {
                Tls::Failed(err) => Err(Self::err_broken_pipe(err)),
                Tls::Ready(ref mut usable) => Ok(usable),
                Tls::Negotiating(_) => Err(Self::err_would_block()),
            },
        }
    }
    /// fix the io if necessary
    fn fix(&mut self, upgrade: bool)
    where
        IO: Read + Write,
    {
        self.io = match self.io.take() {
            None => None,
            Some(tls) => match tls {
                Tls::Failed(_) => Some(tls),
                Tls::Ready(usable) => match usable {
                    Usable::Encrypted(io) => Some(Tls::Ready(Usable::Encrypted(io))),
                    Usable::Plain(io) => {
                        // check if the plaintext IO is OK
                        let upgrade = upgrade && match self.config.mode() {
                            TlsMode::Disabled => false,
                            TlsMode::Enabled => true,
                            TlsMode::StartTlsOptional => self.config.should_start_tls(),
                            TlsMode::StartTlsRquired => self.config.should_start_tls(),
                        };
                        // and upgrade if necessary
                        if upgrade {
                            Some(self.upgrade(io))
                        } else {
                            Some(Tls::Ready(Usable::Plain(io)))
                        }
                    }
                },
                Tls::Negotiating(io) => Some(self.complete(io.handshake())),
            },
        }
    }
    /// upgrade the io to tls
    fn upgrade(&mut self, io: IO) -> Tls<IO>
    where
        IO: Read + Write,
    {
        trace!("Upgrading!");
        match self.try_upgrade(io) {
            Err(e) => Tls::Failed(e),
            Ok(io) => io,
        }
    }
    fn try_upgrade(&mut self, io: IO) -> io::Result<Tls<IO>>
    where
        IO: Read + Write,
    {
        let id = self.config.id();
        match id.file.exists() {
            false => Err(Self::err_no_id_file()),
            true => {
                use std::fs::File;
                let mut f = File::open(id.file)?;
                let mut buf = vec![];
                let _ = f.read_to_end(&mut buf)?;
                let pwd = match id.password {
                    None => "",
                    Some(ref p) => str::from_utf8(p.unsecure()).unwrap_or(""),
                };
                let id = match Identity::from_pkcs12(&buf[..], pwd) {
                    Ok(id) => Ok(id),
                    Err(e) => Err(Self::err_broken_pipe(e)),
                }?;
                match TlsAcceptor::builder(id).build() {
                    Ok(a) => Ok(self.complete(a.accept(io))),
                    Err(e) => Err(Self::err_broken_pipe(e)),
                }
            }
        }
    }
    /// attempt to complete the handshake
    fn complete(
        &mut self,
        handshake: std::result::Result<TlsStream<IO>, HandshakeError<IO>>,
    ) -> Tls<IO> {
        trace!("Completing!");
        match handshake {
            Ok(io) => Tls::Ready(Usable::Encrypted(io)),
            Err(HandshakeError::WouldBlock(io)) => Tls::Negotiating(io),
            Err(HandshakeError::Failure(ref e)) => Tls::Failed(Self::err_broken_pipe(e)),
        }
    }
    fn err_would_block() -> io::Error {
        io::Error::new(io::ErrorKind::WouldBlock, "negotiating tls")
    }
    fn err_no_id_file() -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, "missing id file")
    }
    fn err_no_io() -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, format!("No IO."))
    }
    fn err_broken_pipe(e: impl ToString) -> io::Error {
        io::Error::new(
            io::ErrorKind::BrokenPipe,
            format!("Tls handshake failed. {}", e.to_string()),
        )
    }
}

impl<S> Read for TlsCapable<S>
where
    S: Read + Write,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        trace!("Read!");
        match self.check(false)? {
            Usable::Plain(io) => io.read(buf),
            Usable::Encrypted(io) => io.read(buf),
        }
    }
}

impl<S> Write for TlsCapable<S>
where
    S: Read + Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        trace!("Write!");
        match self.check(false)? {
            Usable::Plain(io) => io.write(buf),
            Usable::Encrypted(io) => io.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        trace!("Flush!");
        match self.check(true)? {
            Usable::Plain(io) => io.flush(),
            Usable::Encrypted(io) => io.flush(),
        }
    }
}

impl<S> AsyncRead for TlsCapable<S>
where
    S: AsyncRead + AsyncWrite,
{
}

impl<S> AsyncWrite for TlsCapable<S>
where
    S: AsyncRead + AsyncWrite,
{
    fn shutdown(&mut self) -> io::Result<Async<()>> {
        match self.check(false)? {
            Usable::Plain(io) => io.shutdown(),
            Usable::Encrypted(io) => match io.shutdown() {
                Ok(()) => Ok(Async::Ready(())),
                Err(e) => Err(e),
            },
        }
    }
}
