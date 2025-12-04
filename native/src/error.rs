#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{error}")]
    User { error: String },
    #[error(transparent)]
    Eyre(#[from] eyre::Report),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Rusqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    TryFromSlice(#[from] std::array::TryFromSliceError),
    #[error(transparent)]
    IrohKeyParsing(#[from] iroh::KeyParsingError),
    #[error(transparent)]
    IrohConnection(#[from] iroh::endpoint::ConnectionError),
    #[error(transparent)]
    IrohWrite(#[from] iroh::endpoint::WriteError),
    #[error(transparent)]
    IrohClosedStream(#[from] iroh::endpoint::ClosedStream),
    #[error(transparent)]
    StringFromUtf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    IrohBind(#[from] iroh::endpoint::BindError),
}
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::User { error: value }
    }
}
