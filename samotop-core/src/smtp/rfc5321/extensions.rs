use crate::smtp::{ExtensionSet, Flag};

impl ExtensionSet {
    pub const PIPELINING: Flag = Flag { code: "PIPELINING" };
    pub const EIGHTBITMIME: Flag = Flag { code: "8BITMIME" };
}
