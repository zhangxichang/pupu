pub trait OptionGet<T> {
    fn get(&self) -> Result<&T, Error>;
    fn get_mut(&mut self) -> Result<&mut T, Error>;
    fn get_move(self) -> Result<T, Error>;
}
impl<T> OptionGet<T> for Option<T> {
    fn get(&self) -> Result<&T, Error> {
        self.as_ref().ok_or("空值".to_string().into())
    }
    fn get_mut(&mut self) -> Result<&mut T, Error> {
        self.as_mut().ok_or("空值".to_string().into())
    }
    fn get_move(self) -> Result<T, Error> {
        self.ok_or("空值".to_string().into())
    }
}
pub trait OptionGetClone<T> {
    fn get_clone(&self) -> Result<T, Error>;
}
impl<T: Clone> OptionGetClone<T> for Option<T> {
    fn get_clone(&self) -> Result<T, Error> {
        self.clone().ok_or("空值".to_string().into())
    }
}

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

#[tauri::command(rename_all = "snake_case")]
pub async fn fatal_error(stack: Option<String>) -> Result<(), Error> {
    log::error!("{}", stack.unwrap_or("没有栈信息".to_string()));
    Ok(())
}
