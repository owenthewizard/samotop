#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SessionShutdown;

// Represents an expired server timeout
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct Timeout;

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct ProcessingError;
