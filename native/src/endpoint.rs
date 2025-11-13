use iroh::SecretKey;

use crate::error::Error;

#[tauri::command(rename_all = "snake_case")]
pub async fn generate_secret_key() -> Result<Vec<u8>, Error> {
    Ok(SecretKey::generate(&mut rand::rng()).to_bytes().to_vec())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn get_secret_key_id(secret_key: Vec<u8>) -> Result<String, Error> {
    Ok(SecretKey::from_bytes(secret_key.as_slice().try_into()?)
        .public()
        .to_string())
}
