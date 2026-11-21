use std::future::Future;

use eyre::{Result, eyre};
use futures_core::Stream;
use futures_util::StreamExt;

pub mod android;
pub mod desktop;
pub mod web;

pub trait FilePickerBackend {
    fn file_name(&self) -> String;
    fn read_stream(&self) -> impl Stream<Item = Result<Vec<u8>>>;
    fn read_to_end(&self) -> impl Future<Output = Result<Vec<u8>>> {
        self.read_to_end_limited(usize::MAX)
    }
    fn read_to_end_limited(&self, max: usize) -> impl Future<Output = Result<Vec<u8>>> {
        async move {
            let mut stream = std::pin::pin!(self.read_stream());
            let mut bytes = Vec::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                if bytes.len().saturating_add(chunk.len()) > max {
                    return Err(eyre!("文件超过大小限制"));
                }
                bytes.extend_from_slice(&chunk);
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
