use crate::smtp::{ExtensionSet, Flag};

impl ExtensionSet {
    pub const STARTTLS: Flag = Flag { code: "STARTTLS" };
}
