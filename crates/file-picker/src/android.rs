#![cfg(target_os = "android")]

use std::{
    ffi::c_void,
    os::fd::OwnedFd,
    sync::{
        Arc,
        atomic::{AtomicPtr, Ordering},
    },
    time::Duration,
};

use eyre::{Result, eyre};
use futures_core::Stream;
use parking_lot::Mutex;
use tokio::{fs::File, io::AsyncReadExt, sync::oneshot};

use crate::{FilePickerBackend, PickOptions};

static PENDING_PICK: Mutex<Option<oneshot::Sender<Result<Option<PickedFile>>>>> = Mutex::new(None);
static ACTIVITY: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn set_activity(activity: *mut c_void) {
    ACTIVITY.store(activity, Ordering::Release);
}

pub enum AndroidPickStatus {
    Picked { fd: OwnedFd, name: String },
    Canceled,
    Failed,
}

#[derive(Clone)]
pub struct PickedFile {
    fd: Arc<OwnedFd>,
    name: String,
}
impl PickedFile {
    pub async fn new(options: PickOptions) -> Result<Option<Self>> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = PENDING_PICK.lock();
            if pending.is_some() {
                return Err(eyre!("已有文件选择正在进行"));
            }
            *pending = Some(sender);
        }
        let result = {
            let mime_types = options.mime_types.join(",");
            let context = ndk_context::android_context();
            let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
            let activity_ptr = ACTIVITY.load(Ordering::Acquire);
            if activity_ptr.is_null() {
                return Err(eyre!("Android Activity 尚未初始化"));
            }
            vm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = unsafe { jni::objects::JObject::from_raw(env, activity_ptr.cast()) };
                let mime_types = env.new_string(mime_types)?;
                env.call_method(
                    &activity,
                    jni::jni_str!("startFilePicker"),
                    jni::jni_sig!("(Ljava/lang/String;)V"),
                    &[jni::JValue::Object(&mime_types)],
                )?;
                Ok(())
            })
        };
        if let Err(err) = result {
            log::warn!("打开文件选择器失败：{err}");
            _ = PENDING_PICK.lock().take();
            return Err(eyre!("打开文件选择器失败：{err}"));
        }
        match tokio::time::timeout(Duration::from_secs(120), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(eyre!("文件选择已结束")),
            Err(_) => {
                _ = PENDING_PICK.lock().take();
                Err(eyre!("文件选择超时"))
            }
        }
    }
}
impl FilePickerBackend for PickedFile {
    fn file_name(&self) -> String {
        self.name.clone()
    }

    fn read_stream(&self) -> impl Stream<Item = Result<Vec<u8>>> {
        async_stream::try_stream! {
            let mut file = File::from(self.fd.try_clone()?);
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

pub fn handle_android_file_pick(status: AndroidPickStatus) {
    let sender = PENDING_PICK.lock().take();
    let Some(sender) = sender else {
        return;
    };
    let result = match status {
        AndroidPickStatus::Picked { fd, name } => Ok(Some(PickedFile {
            fd: Arc::new(fd),
            name,
        })),
        AndroidPickStatus::Canceled => Ok(None),
        AndroidPickStatus::Failed => Err(eyre!("打开文件失败")),
    };
    _ = sender.send(result);
}
