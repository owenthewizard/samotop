use crate::{io::Io, config::Store};
pub struct Session {
    pub io: Box<dyn Io>,
    pub store: Store,
}
impl Session {
    pub fn new(io: impl Io + 'static) -> Self {
        Self {
            io: Box::new(io),
            store: Store::default(),
        }
    }
}
