use std::collections::HashMap;

#[derive(Eq, PartialEq, Debug, Clone, Hash)]
pub struct SmtpExtension {
    pub code: &'static str,
    pub params: Vec<String>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ExtensionSet {
    map: HashMap<&'static str, SmtpExtension>,
}

impl ExtensionSet {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &SmtpExtension> {
        self.map.values()
    }
    pub fn get<'s>(&'s self, code: &str) -> Option<&'s SmtpExtension> {
        self.map.get(code)
    }
    pub fn is_enabled(&self, code: &str) -> bool {
        self.map.contains_key(code)
    }
    pub fn enable(&mut self, extension: SmtpExtension) -> bool {
        self.map.insert(extension.code, extension).is_some()
    }
    pub fn disable(&mut self, code: &str) -> bool {
        self.map.remove(code).is_some()
    }
}

impl SmtpExtension {
    pub const STARTTLS: Self = Self {
        code: "STARTTLS",
        params: vec![],
    };
    pub const PIPELINING: Self = Self {
        code: "PIPELINING",
        params: vec![],
    };
    pub const EIGHTBITMIME: Self = Self {
        code: "8BITMIME",
        params: vec![],
    };
}

impl std::fmt::Display for SmtpExtension {
    fn fmt<'a>(&self, fmt: &'a mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        fmt.write_str(self.code)?;
        for param in self.params.iter() {
            fmt.write_str(" ")?;
            fmt.write_str(param)?;
        }
        Ok(())
    }
}
