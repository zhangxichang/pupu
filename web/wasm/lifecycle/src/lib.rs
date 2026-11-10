// TODO AI实现待审查

//! One-shot readiness signal for a wasm module's exported `start()`.
//!
//! Each wasm instance has its own state. A module's exported `start()` awaits
//! [`wait`], and the module calls [`notify_ready`] at the point where it is
//! ready. If it fails before that point, call [`notify_error`].
//!
//! ```ignore
//! #[wasm_bindgen]
//! pub async fn start() -> Result<(), JsError> {
//!     wasm_bindgen_futures::spawn_local(async {
//!         // Run the module's initialization.
//!         lifecycle::notify_ready();
//!     });
//!
//!     lifecycle::wait().await
//! }
//! ```

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use wasm_bindgen::JsError;

struct State {
    result: Option<Result<(), String>>,
    waker: Option<Waker>,
}

thread_local! {
    static STATE: RefCell<State> = const {
        RefCell::new(State {
            result: None,
            waker: None,
        })
    };
}

/// Marks the module as ready, resolving [`wait`] with `Ok(())`.
pub fn notify_ready() {
    notify(Ok(()));
}

/// Marks the module as failed, resolving [`wait`] with an `Err(JsError)`.
pub fn notify_error(message: impl Into<String>) {
    notify(Err(message.into()));
}

/// Waits until [`notify_ready`] or [`notify_error`] is called.
///
/// Call this once from the module's exported `start()`.
pub fn wait() -> Wait {
    Wait
}

/// The future returned by [`wait`].
#[must_use = "futures do nothing unless awaited"]
pub struct Wait;

impl Future for Wait {
    type Output = Result<(), JsError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = STATE.with(|state| {
            let mut state = state.borrow_mut();
            match state.result.take() {
                Some(result) => Some(result.map_err(|message| JsError::new(&message))),
                None => {
                    state.waker = Some(cx.waker().clone());
                    None
                }
            }
        });

        match result {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

fn notify(result: Result<(), String>) {
    let waker = STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.result.is_some() {
            return None;
        }
        state.result = Some(result);
        state.waker.take()
    });

    if let Some(waker) = waker {
        waker.wake();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll, Wake, Waker},
    };

    use super::*;

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    struct RecordingWake(Arc<AtomicBool>);

    impl Wake for RecordingWake {
        fn wake(self: Arc<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn poll_wait() -> Poll<Result<(), JsError>> {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = wait();
        Pin::new(&mut future).poll(&mut context)
    }

    #[test]
    fn ready_resolves_wait() {
        notify_ready();
        assert!(matches!(poll_wait(), Poll::Ready(Ok(()))));
    }

    #[test]
    fn error_message_is_recorded() {
        notify_error("boom");
        let result = STATE.with(|state| state.borrow_mut().result.take());
        assert!(matches!(result, Some(Err(message)) if message == "boom"));
    }

    #[test]
    fn wait_is_pending_until_notified() {
        let woke = Arc::new(AtomicBool::new(false));
        let waker = Waker::from(Arc::new(RecordingWake(Arc::clone(&woke))));
        let mut context = Context::from_waker(&waker);
        let mut future = wait();

        assert!(matches!(
            Pin::new(&mut future).poll(&mut context),
            Poll::Pending
        ));
        assert!(!woke.load(Ordering::SeqCst));

        notify_ready();

        assert!(woke.load(Ordering::SeqCst));
        assert!(matches!(
            Pin::new(&mut future).poll(&mut context),
            Poll::Ready(Ok(()))
        ));
    }
}
