pub trait TauriState {
    fn tauri_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R>;
}

pub fn tauri_state_builder<T, R>(builder: tauri::Builder<R>) -> tauri::Builder<R>
where
    T: TauriState + Default + Send + Sync + 'static,
    R: tauri::Runtime,
{
    T::tauri_builder(builder.manage(T::default()))
}
