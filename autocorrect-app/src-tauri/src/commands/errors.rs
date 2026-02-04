#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    #[error("Input simulation error: {0}")]
    InputSimulation(String),
    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(serde::Serialize)]
#[serde(tag = "name", content = "message")]
#[serde(rename_all = "camelCase")]
enum ErrorName {
    Io(String),
    FromUtf8Error(String),
    Clipboard(String),
    InputSimulation(String),
    Config(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let message = self.to_string();
        let name = match self {
            Self::Io(_) => ErrorName::Io(message),
            Self::Utf8(_) => ErrorName::FromUtf8Error(message),
            Self::Clipboard(_) => ErrorName::Clipboard(message),
            Self::InputSimulation(_) => ErrorName::InputSimulation(message),
            Self::Config(_) => ErrorName::Config(message),
        };
        name.serialize(serializer)
    }
}
