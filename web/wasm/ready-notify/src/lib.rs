use std::cell::RefCell;

use futures_channel::oneshot;
use wasm_bindgen::JsError;

struct Signal {
    tx: Option<oneshot::Sender<Result<(), String>>>,
    rx: Option<oneshot::Receiver<Result<(), String>>>,
}

thread_local! {
    static SIGNAL: RefCell<Signal> = RefCell::new({
        let (tx, rx) = oneshot::channel();
        Signal {
            tx: Some(tx),
            rx: Some(rx),
        }
    });
}

pub fn notify_ready() {
    notify(Ok(()));
}

pub fn notify_error(message: impl Into<String>) {
    notify(Err(message.into()));
}

pub async fn wait() -> Result<(), JsError> {
    wait_result()
        .await
        .map_err(|message| JsError::new(&message))
}

async fn wait_result() -> Result<(), String> {
    let rx = SIGNAL.with(|signal| signal.borrow_mut().rx.take());
    let Some(rx) = rx else {
        return Err("ready-notify 信号已经等待过了".into());
    };
    match rx.await {
        Ok(result) => result,
        Err(_) => Err("ready-notify 信号发送端已关闭".into()),
    }
}

fn notify(result: Result<(), String>) {
    if let Some(tx) = SIGNAL.with(|signal| signal.borrow_mut().tx.take()) {
        let _ = tx.send(result);
    }
}
