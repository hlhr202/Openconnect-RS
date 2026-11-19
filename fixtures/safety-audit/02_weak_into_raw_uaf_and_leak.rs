//! Fixture 02: Proves `VpnClient::new` + `VpnClient::ref_from_raw` bugs:
//!   (a) `Weak::into_raw` leaks `ArcInner` allocation forever (never paired with `Weak::from_raw`).
//!   (b) `VpnClient::ref_from_raw(privdata)` dereferences a dropped `VpnClient` (Use-After-Free)
//!       once the last strong `Arc<VpnClient>` is dropped while C still holds `privdata`.
//!   (c) Writing `(*raw_instance).vpninfo = vpninfo` mutates through a shared `Weak::into_raw`
//!       pointer without `UnsafeCell` (Stacked/Tree Borrows UB).

use std::alloc::{GlobalAlloc, Layout, System};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

struct TrackingAllocator;

static LIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            LIVE_ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
            LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_ALLOCATIONS.fetch_sub(1, Ordering::SeqCst);
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator;

struct DropTracker {
    dropped: Arc<AtomicBool>,
    magic: u64,
}

impl Drop for DropTracker {
    fn drop(&mut self) {
        self.magic = 0xDEAD_DEAD_DEAD_DEAD;
        self.dropped.store(true, Ordering::SeqCst);
    }
}

/// Mirrors `crates/openconnect-core/src/lib.rs:69-78`
#[repr(C)]
struct BuggyVpnClient {
    vpninfo: *mut c_void,
    tracker: DropTracker,
    payload: [u8; 4096],
}

impl BuggyVpnClient {
    /// Mirrors `crates/openconnect-core/src/lib.rs:471-507`
    fn new(dropped_flag: Arc<AtomicBool>) -> (Arc<Self>, *mut c_void) {
        let instance = Arc::new(Self {
            vpninfo: std::ptr::null_mut(),
            tracker: DropTracker {
                dropped: dropped_flag,
                magic: 0x1122_3344_5566_7788,
            },
            payload: [0xAB; 4096],
        });

        let privdata = unsafe {
            let weak_instance = Arc::downgrade(&instance);
            let raw_instance = Weak::into_raw(weak_instance) as *mut BuggyVpnClient;
            // BUG (c): Writing through `*const T` cast to `*mut T` without `UnsafeCell` (UB!)
            (*raw_instance).vpninfo = 0x1234usize as *mut c_void;
            raw_instance as *mut c_void
        };

        (instance, privdata)
    }

    /// Mirrors `crates/openconnect-core/src/lib.rs:112-115`
    unsafe fn ref_from_raw<'a>(ptr: *mut c_void) -> &'a Self {
        let ptr = ptr.cast::<Self>();
        &*ptr
    }
}

fn main() {
    let dropped_flag = Arc::new(AtomicBool::new(false));
    let before_allocs = LIVE_ALLOCATIONS.load(Ordering::SeqCst);
    let before_bytes = LIVE_BYTES.load(Ordering::SeqCst);

    let (client, c_privdata) = BuggyVpnClient::new(dropped_flag.clone());
    let weak_count_while_alive = Arc::weak_count(&client);
    println!("[02] weak_count while Arc alive = {}", weak_count_while_alive);

    // Drop the last strong Arc<BuggyVpnClient>.
    drop(client);

    let after_allocs = LIVE_ALLOCATIONS.load(Ordering::SeqCst);
    let after_bytes = LIVE_BYTES.load(Ordering::SeqCst);

    assert!(dropped_flag.load(Ordering::SeqCst), "Drop must have run");
    assert!(
        after_bytes > before_bytes,
        "Expected ArcInner leak after dropping Arc"
    );
    println!(
        "[02] PROVEN (Leak): Arc<VpnClient> dropped, yet {} allocation(s) / {} bytes remain leaked because Weak::into_raw is never reclaimed!",
        after_allocs - before_allocs,
        after_bytes - before_bytes
    );

    // Now simulate a C callback (e.g., stats_fn / validate_peer_cert / default_setup_tun_vfn)
    // firing with `c_privdata` after VpnClient has already been dropped:
    let uaf_ref = unsafe { BuggyVpnClient::ref_from_raw(c_privdata) };
    assert_eq!(uaf_ref.tracker.magic, 0xDEAD_DEAD_DEAD_DEAD);
    println!(
        "[02] PROVEN (UAF): ref_from_raw(privdata) returned `&VpnClient` pointing to ALREADY-DROPPED memory (magic=0x{:X})!",
        uaf_ref.tracker.magic
    );
}
