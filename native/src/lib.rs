mod error;
mod state;
mod tauri_plugin_builder;
mod tauri_state_builder;

use crate::{
    state::State, tauri_plugin_builder::tauri_plugin_builder,
    tauri_state_builder::tauri_state_builder,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri_state_builder::<State, _>(tauri_plugin_builder(tauri::Builder::default()))
        .run(tauri::generate_context!())
        .unwrap();
}
