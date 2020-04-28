use crate::model::controll::*;

pub fn tls_capable<IO>(io: IO, _config: TlsWorker) -> TlsCapable<IO> {
    io
}

pub type TlsCapable<IO> = IO;
