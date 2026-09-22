use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Engine(String),
    Decode(String),
    VaultUnreadable(String),
    BrowserOnly,
}

impl Failure {
    pub fn reported(&self) -> String {
        match self {
            Failure::Engine(message) => message.clone(),
            Failure::Decode(message) => message.clone(),
            Failure::VaultUnreadable(message) => message.clone(),
            Failure::BrowserOnly => "File selection requires the app frontend".to_owned(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reported())
    }
}

impl std::error::Error for Failure {}
