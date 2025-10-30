mod database;

use crate::{state::database::Database, tauri_state_builder::TauriState};

#[derive(Default)]
pub struct State {
    database: Database,
}
impl TauriState for State {
    fn tauri_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
        builder.invoke_handler(tauri::generate_handler![])
    }
}
