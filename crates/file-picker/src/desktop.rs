#![cfg(any(target_os = "windows", target_os = "linux"))]

use std::path::PathBuf;

use eyre::Result;
use futures_core::Stream;
use tokio::{fs::File, io::AsyncReadExt};

use crate::{FilePickerBackend, PickOptions};

#[derive(Clone)]
pub struct PickedFile {
    path: PathBuf,
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
        Ok(Some(Self {
            path: handle.path().to_path_buf(),
        }))
    }
}
impl FilePickerBackend for PickedFile {
    fn read_stream(&self) -> impl Stream<Item = Result<Vec<u8>>> {
        async_stream::try_stream! {
            let mut file = File::open(&self.path).await?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer).await?;
                if read == 0 {
                    break;
                }
                yield buffer[..read].to_vec();
            }
        }
    }
}
