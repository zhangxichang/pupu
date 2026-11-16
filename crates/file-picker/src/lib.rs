use std::future::Future;

use eyre::Result;
use futures_core::Stream;
use futures_util::StreamExt;

pub mod android;
pub mod desktop;
pub mod web;

pub trait FilePickerBackend {
    fn read_stream(&self) -> impl Stream<Item = Result<Vec<u8>>>;
    fn read_to_end(&self) -> impl Future<Output = Result<Vec<u8>>> {
        async move {
            let mut stream = std::pin::pin!(self.read_stream());
            let mut bytes = Vec::new();
            while let Some(chunk) = stream.next().await {
                bytes.extend_from_slice(&chunk?);
            }
            Ok(bytes)
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PickOptions {
    pub title: String,
    pub filters: Vec<FileFilter>,
    pub mime_types: Vec<String>,
}
