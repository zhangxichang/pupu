use crate::error::Error;

pub trait OptionGet<T> {
    fn get(&self) -> Result<&T, Error>;
    fn get_move(self) -> Result<T, Error>;
}
impl<T> OptionGet<T> for Option<T> {
    fn get(&self) -> Result<&T, Error> {
        self.as_ref().ok_or("空值".to_string().into())
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
