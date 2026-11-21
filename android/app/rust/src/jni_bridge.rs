use std::os::fd::{FromRawFd, OwnedFd};

use file_picker::android::{AndroidPickStatus, handle_android_file_pick};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_zhangxichang_pupu_FilePicker_nativeOnFilePicked<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    status: jni::sys::jint,
    fd: jni::sys::jint,
    name: jni::objects::JString<'local>,
) {
    let name = env
        .with_env(|env| -> jni::errors::Result<String> {
            Ok(name.mutf8_chars(env)?.to_str().to_string())
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    let status = match status {
        0 => {
            if fd < 0 {
                AndroidPickStatus::Failed
            } else {
                AndroidPickStatus::Picked {
                    fd: unsafe { OwnedFd::from_raw_fd(fd) },
                    name,
                }
            }
        }
        1 => AndroidPickStatus::Canceled,
        _ => AndroidPickStatus::Failed,
    };
    handle_android_file_pick(status);
}
