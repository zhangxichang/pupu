use crate::error::Error;

#[tauri::command(rename_all = "snake_case")]
pub async fn log_error(error: String) -> Result<(), Error> {
    log::error!("{}", error);
    Ok(())
}
