use tokio::fs::remove_file;

use crate::error::Error;

#[tauri::command(rename_all = "snake_case")]
pub async fn fs_remove_file(path: String) -> Result<(), Error> {
    remove_file(path).await?;
    Ok(())
}
