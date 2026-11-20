//! Fixture 03: Proves `unsafe impl Sync for VpnClient` + `EventHandlers` soundness hole.
//!
//! `crates/openconnect-core/src/events.rs:20-27` only bounds `F: Fn(Status) + Send + 'static`
//! (missing `+ Sync`), while `crates/openconnect-core/src/lib.rs:80-81` declares
//! `unsafe impl Send for VpnClient {}` and `unsafe impl Sync for VpnClient {}`.
//!
//! Result: 100% SAFE caller code can pass a `!Sync` `Cell` / `RefCell` closure into
//! `EventHandlers` and trigger a Data Race / UB across threads!

use std::cell::RefCell;
use std::sync::{Arc, Barrier};
use std::thread;

/// Exact reproduction of `crates/openconnect-core/src/events.rs:7-27`
#[derive(Clone)]
struct BuggyEventHandlers {
    handle_connection_state_change: Option<Arc<dyn Fn(usize)>>,
}

impl BuggyEventHandlers {
    fn new() -> Self {
        Self {
            handle_connection_state_change: None,
        }
    }

    fn with_handle_connection_state_change<F>(mut self, handler: F) -> Self
    where
        F: Fn(usize),
        F: Send + 'static, // <-- BUG: missing `+ Sync`!
    {
        self.handle_connection_state_change = Some(Arc::new(handler));
        self
    }
}

/// Exact reproduction of `crates/openconnect-core/src/lib.rs:69-81`
struct BuggyVpnClient {
    callbacks: BuggyEventHandlers,
}

unsafe impl Send for BuggyVpnClient {}
unsafe impl Sync for BuggyVpnClient {}

impl BuggyVpnClient {
    fn emit_state_change(&self, state: usize) {
        if let Some(ref handler) = self.callbacks.handle_connection_state_change {
            handler(state);
        }
    }
}

fn main() {
    // 100% safe caller code using `RefCell<usize>` (which is Send, !Sync).
    // Thread 1 acquires `borrow_mut()`, signals thread 2, and yields while holding the guard.
    // Thread 2 concurrently calls `emit_state_change` on the shared `Arc<BuggyVpnClient>`
    // and attempts `borrow_mut()` -> deterministic BorrowMutError panic in 100% safe caller code!
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    let entered = Arc::new(AtomicBool::new(false));
    let entered_cb = entered.clone();
    let state = RefCell::new(0usize);

    let handlers = BuggyEventHandlers::new().with_handle_connection_state_change(move |x| {
        let mut guard = state.borrow_mut();
        *guard = guard.wrapping_add(x);
        if !entered_cb.swap(true, Ordering::SeqCst) {
            // First thread holds the RefCell mutable borrow while thread 2 enters.
            thread::sleep(Duration::from_millis(50));
        }
    });

    let client = Arc::new(BuggyVpnClient { callbacks: handlers });
    let barrier = Arc::new(Barrier::new(2));

    let c1 = client.clone();
    let b1 = barrier.clone();
    let t1 = thread::spawn(move || {
        b1.wait();
        c1.emit_state_change(1);
    });

    let c2 = client.clone();
    let b2 = barrier.clone();
    let t2 = thread::spawn(move || {
        b2.wait();
        while !entered.load(Ordering::SeqCst) {
            thread::yield_now();
        }
        c2.emit_state_change(2);
    });

    let r1 = t1.join();
    let r2 = t2.join();
    let panicked = usize::from(r1.is_err()) + usize::from(r2.is_err());

    assert!(
        panicked >= 1,
        "Expected deterministic concurrent RefCell::borrow_mut() panic in safe caller code"
    );
    println!(
        "[03] PROVEN (Unsound Sync): {}/2 threads panicked on concurrent RefCell::borrow_mut() in 100% safe caller code due to `unsafe impl Sync for VpnClient` without `F: Sync`!",
        panicked
    );
}
