#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Invalid theme value: {0}")]
    InvalidTheme(String),
    #[error("Theme store error: {0}")]
    Store(String),
    #[error("Theme sync error: {0}")]
    Sync(String),
}

#[derive(serde::Serialize)]
#[serde(tag = "name", content = "message")]
#[serde(rename_all = "camelCase")]
enum ThemeErrorName {
    InvalidTheme(String),
    Store(String),
    Sync(String),
}

impl serde::Serialize for ThemeError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let message = self.to_string();
        let name = match self {
            Self::InvalidTheme(_) => ThemeErrorName::InvalidTheme(message),
            Self::Store(_) => ThemeErrorName::Store(message),
            Self::Sync(_) => ThemeErrorName::Sync(message),
        };
        name.serialize(serializer)
    }
}
