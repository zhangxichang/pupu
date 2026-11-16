#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use eyre::{Result, eyre};
use futures_core::Stream;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, js_sys, wasm_bindgen::JsCast};

use crate::{FilePickerBackend, PickOptions};

#[derive(Clone)]
pub struct PickedFile {
    handle: rfd::FileHandle,
}
impl PickedFile {
    pub async fn new(options: PickOptions) -> Result<Option<Self>> {
        let mut dialog = rfd::AsyncFileDialog::new().set_title(options.title);
        for filter in options.filters {
            dialog = dialog.add_filter(filter.name, &filter.extensions);
        }
        let Some(handle) = dialog.pick_file().await else {
            return Ok(None);
        };
        Ok(Some(Self { handle }))
    }
}
impl FilePickerBackend for PickedFile {
    fn read_stream(&self) -> impl Stream<Item = Result<Vec<u8>>> {
        async_stream::try_stream! {
            const CHUNK_SIZE: f64 = 64.0 * 1024.0;
            let blob = self.handle.inner().unchecked_ref::<Blob>();
            let mut offset = 0.0;
            while offset < blob.size() {
                let end = (offset + CHUNK_SIZE).min(blob.size());
                let chunk = blob
                    .slice_with_f64_and_f64(offset, end)
                    .map_err(|err| eyre!("读取文件失败：{err:?}"))?;
                let array_buffer = JsFuture::from(chunk.array_buffer())
                    .await
                    .map_err(|err| eyre!("读取文件失败：{err:?}"))?;
                let array = js_sys::Uint8Array::new(&array_buffer);
                let mut bytes = vec![0; array.length() as usize];
                array.copy_to(&mut bytes);
                offset = end;
                yield bytes;
            }
        }
    }
}
