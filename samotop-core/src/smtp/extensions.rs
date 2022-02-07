use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtRes};

pub trait Extension: Display {
    type Value: ExtensionValue;
    fn parse(&self, input: &str) -> Result<Option<Self::Value>, Error>;
}
pub trait ExtensionValue: Display + Debug + Clone {
    type Extension: Extension;
    fn extension(&self) -> &Self::Extension;
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub enum Error {
    Incomplete,
    Invalid(usize),
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        write!(f, "Parsing SMTP extension failed. ")?;
        match self {
            Error::Incomplete => write!(f, "The input is incomplete."),
            Error::Invalid(at) => write!(f, "The input is invalid at {}.", at),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Default)]
pub struct ExtensionSet {
    map: HashMap<String, String>,
}

impl ExtensionSet {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.map.values().map(|s| s.as_str())
    }
    pub fn get<T: Extension>(&self, ext: &T) -> Result<Option<T::Value>, Error> {
        self.get_string(ext.to_string().as_str())
            .map(|s| ext.parse(s))
            .transpose()
            .map(Option::flatten)
    }
    pub fn get_string<'s>(&'s self, code: &str) -> Option<&'s str> {
        self.map.get(code).map(|s| s.as_str())
    }
    pub fn is_enabled<T: Extension>(&self, ext: &T) -> bool {
        self.is_enabled_code(ext.to_string().as_str())
    }
    pub fn is_enabled_code(&self, code: &str) -> bool {
        self.map.contains_key(code)
    }
    pub fn enable<T: ExtensionValue>(&mut self, ext: &T) -> bool {
        self.enable_string(
            ext.extension().to_string().as_str(),
            ext.to_string().as_str(),
        )
    }
    pub fn enable_string(&mut self, code: &str, ext: &str) -> bool {
        self.map.insert(code.to_string(), ext.to_string()).is_some()
    }
    pub fn disable<T: Extension>(&mut self, ext: &T) -> bool {
        self.disable_code(ext.to_string().as_str())
    }
    pub fn disable_code(&mut self, code: &str) -> bool {
        self.map.remove(code).is_some()
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct Flag {
    pub code: &'static str,
}
impl Extension for Flag {
    type Value = Self;
    fn parse(&self, input: &str) -> Result<Option<Self>, Error> {
        match input == self.code {
            true => Ok(Some(*self)),
            false => match self.code.starts_with(input) {
                // The input is part of the code, but too short
                true => Err(Error::Incomplete),
                false => match input.starts_with(self.code) {
                    false => Ok(None),
                    true => match &input.as_bytes()[self.code.len()..] {
                        // input starts with our code but it is a different longer word
                        [b'a'..=b'z', ..] | [b'A'..=b'Z', ..] | [b'0'..=b'9', ..] => Ok(None),
                        // this is meant to be an extension without params,
                        // but apparently there are some params
                        // or other mess
                        _ => Err(Error::Invalid(self.code.len())),
                    },
                },
            },
        }
    }
}
impl ExtensionValue for Flag {
    type Extension = Self;
    fn extension(&self) -> &Self::Extension {
        self
    }
}
impl Display for Flag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        f.write_str(self.code)
    }
}

#[cfg(test)]
mod extension_set {
    use super::*;

    #[test]
    fn enable_extension() {
        let mut sut = ExtensionSet::new();
        // extension is not enabled yet so enable returns false
        assert!(!sut.enable(&ExtensionSet::STARTTLS));
        // extension is already enabled so enable returns true
        assert!(sut.enable(&ExtensionSet::STARTTLS));
    }
    #[test]
    fn disable_extension() {
        let mut sut = ExtensionSet::new();
        sut.enable(&ExtensionSet::STARTTLS);
        // extension is enabled so disable returns true
        assert!(sut.disable(&ExtensionSet::STARTTLS));
        // extension is not enabled anymore so disable returns false
        assert!(!sut.disable(&ExtensionSet::STARTTLS));
    }
    #[test]
    fn get_extension() {
        let mut sut = ExtensionSet::new();
        // extension is disabled so gives None
        assert_eq!(sut.get(&ExtensionSet::STARTTLS).unwrap(), None);
        sut.enable(&ExtensionSet::STARTTLS);
        // extension is enabled so gives Some
        assert_eq!(
            sut.get(&ExtensionSet::STARTTLS).unwrap(),
            Some(ExtensionSet::STARTTLS)
        );
    }
    #[test]
    fn check_extension() {
        let mut sut = ExtensionSet::new();
        // extension is disabled so gives None
        assert!(!sut.is_enabled(&ExtensionSet::STARTTLS));
        sut.enable(&ExtensionSet::STARTTLS);
        // extension is enabled so gives Some
        assert!(sut.is_enabled(&ExtensionSet::STARTTLS));
    }
}

#[cfg(test)]
mod flag_parsing {
    use super::*;

    #[test]
    fn parse_starttls() {
        assert_eq!(
            ExtensionSet::STARTTLS.parse("STARTTLS").unwrap().unwrap(),
            ExtensionSet::STARTTLS
        );
    }
    #[test]
    fn parse_incomplete() {
        assert_eq!(
            ExtensionSet::STARTTLS.parse("STARTT").unwrap_err(),
            Error::Incomplete
        );
    }
    #[test]
    fn parse_invalid() {
        assert_eq!(
            ExtensionSet::STARTTLS.parse("STARTTLS ").unwrap_err(),
            Error::Invalid(8)
        );
        assert_eq!(
            ExtensionSet::STARTTLS.parse("STARTTLS\t").unwrap_err(),
            Error::Invalid(8)
        );
        assert_eq!(
            ExtensionSet::STARTTLS.parse("STARTTLS param").unwrap_err(),
            Error::Invalid(8)
        );
    }
    #[test]
    fn parse_mismatch() {
        assert_eq!(ExtensionSet::STARTTLS.parse("OTHER").unwrap(), None);
        assert_eq!(ExtensionSet::STARTTLS.parse("STARTTLSx").unwrap(), None);
    }
}
