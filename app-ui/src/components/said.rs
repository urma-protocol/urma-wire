#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    Something(String),
    Nothing,
}

impl From<Option<String>> for Said {
    fn from(message: Option<String>) -> Self {
        let Some(message) = message else {
            return Said::Nothing;
        };
        Said::Something(message)
    }
}
