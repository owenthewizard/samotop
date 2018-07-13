use secstr;
use std::mem;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, SendError, Sender, TryRecvError};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsMode {
    Disabled,
    Enabled,
    StartTlsOptional,
    StartTlsRquired,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TlsIdFile {
    pub file: PathBuf,
    pub password: Option<secstr::SecStr>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TlsConfig {
    pub mode: TlsMode,
    pub id: TlsIdFile,
}

impl Default for TlsConfig {
    fn default() -> Self {
        let mode = if cfg!(feature = "tls") {
            TlsMode::StartTlsOptional
        } else {
            TlsMode::Disabled
        };
        Self {
            mode,
            id: TlsIdFile {
                file: PathBuf::from("Samotop.pfx"),
                password: None,
            },
        }
    }
}

impl TlsConfig {
    pub fn parts(&self) -> (TlsControll, TlsWorker) {
        let (stls_tx, stls_rx) = channel();
        (
            TlsControll::new(stls_tx, self.mode),
            TlsWorker::new(stls_rx, self.mode, self.id.clone()),
        )
    }
    pub fn check_identity(self) -> Self {
        if self.mode == TlsMode::Disabled {
            return self;
        }

        let Self { mut mode, id } = self;

        if !id.file.exists() {
            warn!(
                "Identity file is missing: {:?}. TLS will be disabled.",
                id.file
            );
            mode = TlsMode::Disabled;
        }

        Self { id, mode }
    }
}

pub struct TlsWorker {
    signal: Receiver<()>,
    mode: TlsMode,
    id: TlsIdFile,
}

impl TlsWorker {
    pub fn new(signal: Receiver<()>, mode: TlsMode, id: TlsIdFile) -> Self {
        Self { signal, mode, id }
    }
    pub fn id(&mut self) -> TlsIdFile {
        self.id.clone()
    }
    pub fn mode(&mut self) -> TlsMode {
        let mode = match self.mode {
            TlsMode::Disabled => TlsMode::Disabled,
            TlsMode::Enabled => TlsMode::Enabled,
            m => match self.signal.try_recv() {
                Ok(()) => TlsMode::Enabled,
                Err(TryRecvError::Empty) => m,
                Err(TryRecvError::Disconnected) => TlsMode::Disabled,
            },
        };
        mem::replace(&mut self.mode, mode);
        self.mode
    }
    pub fn should_start_tls(&mut self) -> bool {
        match self.mode() {
            TlsMode::Enabled => true,
            _ => false,
        }
    }
}

pub struct TlsControll {
    sender: Sender<()>,
    mode: TlsMode,
}

impl TlsControll {
    pub fn new(sender: Sender<()>, mode: TlsMode) -> Self {
        Self { sender, mode }
    }
    pub fn mode(&self) -> TlsMode {
        self.mode
    }
    pub fn start_tls(&self) {
        match self.mode {
            TlsMode::Disabled => {}
            _ => match self.sender.send(()) {
                Ok(()) => {}
                Err(SendError(())) => {}
            },
        }
    }
}
