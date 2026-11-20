use crate::{
    SharedState,
    log::Logger,
    result::{OpenconnectError, OpenconnectResult},
};
use openconnect_sys::{
    helper_format_vargs, helper_set_global_progress_vfn, openconnect_info, openconnect_init_ssl,
    openconnect_vpninfo_free, openconnect_vpninfo_new,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr::NonNull,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicPtr, Ordering},
    },
};

static GLOBAL_OC_INIT: OnceLock<Result<(), i32>> = OnceLock::new();

/// Initialize OpenConnect SSL subsystem and global progress callback exactly once.
pub(crate) fn ensure_global_init() -> OpenconnectResult<()> {
    let result = GLOBAL_OC_INIT.get_or_init(|| {
        // SAFETY: Called once via `OnceLock`, avoiding concurrent initialization races in OpenSSL/libopenconnect.
        let ret = unsafe { openconnect_init_ssl() };
        if ret != 0 {
            return Err(ret);
        }
        // SAFETY: `helper_set_global_progress_vfn` stores the function pointer atomically.
        unsafe {
            helper_set_global_progress_vfn(Some(Logger::raw_handle_process_log));
        }
        Ok(())
    });

    match result {
        Ok(()) => Ok(()),
        Err(code) => Err(OpenconnectError::InitSslError(*code)),
    }
}

/// Convert a nullable C string pointer into `Option<String>` using lossy UTF-8 conversion.
///
/// # Safety
/// If `ptr` is non-null, it must point to a valid, readable, NUL-terminated C string
/// for the duration of this call.
pub(crate) unsafe fn c_str_to_option_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: Caller guarantees `ptr` points to a valid NUL-terminated C string.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    Some(cstr.to_string_lossy().into_owned())
}

/// Execute a closure inside an FFI callback, catching any Rust panic so unwinding
/// never crosses an `extern "C"` boundary (which is Undefined Behavior).
pub(crate) fn ffi_catch_unwind<R>(default: R, f: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Caught panic inside C FFI callback; returning fail-safe default");
            default
        }
    }
}

/// Owned callback context passed as `privdata` to `libopenconnect`.
///
/// Ownership invariant:
/// - Allocated via `Box::into_raw` in [`VpnInfoHandle::new`].
/// - Owned exclusively by [`VpnInfoHandle`].
/// - Freed strictly AFTER `openconnect_vpninfo_free` returns in [`VpnInfoHandle::drop`],
///   guaranteeing no C callback can ever observe a dangling `privdata` pointer.
pub(crate) struct CallbackContext {
    pub(crate) state: Arc<SharedState>,
    vpninfo: AtomicPtr<openconnect_info>,
}

impl CallbackContext {
    pub(crate) fn vpninfo_ptr(&self) -> Option<NonNull<openconnect_info>> {
        NonNull::new(self.vpninfo.load(Ordering::Acquire))
    }
}

/// Safely borrow [`CallbackContext`] from an FFI `privdata` pointer while catching panics.
///
/// # Safety
/// If `privdata` is non-null, it must be a live pointer previously created by
/// [`VpnInfoHandle::new`] (i.e. called synchronously from `libopenconnect` before
/// `openconnect_vpninfo_free` completes).
pub(crate) unsafe fn with_callback_context<R: Copy>(
    privdata: *mut c_void,
    default: R,
    f: impl FnOnce(&CallbackContext) -> R,
) -> R {
    ffi_catch_unwind(default, || {
        if privdata.is_null() {
            tracing::error!("Received NULL privdata in C FFI callback");
            return default;
        }
        // SAFETY: Caller guarantees `privdata` points to a valid `CallbackContext` owned
        // by a live `VpnInfoHandle` for the entire duration of this callback.
        let ctx = unsafe { &*privdata.cast::<CallbackContext>() };
        f(ctx)
    })
}

/// Exclusive RAII owner of `openconnect_info` and its heap-allocated [`CallbackContext`].
///
/// `VpnInfoHandle` is `Send` (ownership can be moved across threads under a `Mutex`),
/// and intentionally `!Sync` because `libopenconnect`'s `struct openconnect_info` is
/// not thread-safe for concurrent access.
pub(crate) struct VpnInfoHandle {
    vpninfo: NonNull<openconnect_info>,
    ctx_ptr: NonNull<CallbackContext>,
    state: Arc<SharedState>,
}

// SAFETY: `openconnect_info` and `CallbackContext` can be transferred across threads
// as long as all accesses are serialized (enforced because `VpnInfoHandle` is `!Sync`
// and wrapped in a `Mutex`).
unsafe impl Send for VpnInfoHandle {}

impl VpnInfoHandle {
    pub(crate) fn new(useragent: &CString, state: Arc<SharedState>) -> OpenconnectResult<Self> {
        ensure_global_init()?;

        let ctx_box = Box::new(CallbackContext {
            state: Arc::clone(&state),
            vpninfo: AtomicPtr::new(std::ptr::null_mut()),
        });
        let ctx_raw = Box::into_raw(ctx_box);
        let ctx_ptr = NonNull::new(ctx_raw).expect("Box::into_raw never returns null");

        // SAFETY:
        // - `useragent.as_ptr()` is a valid NUL-terminated C string (`libopenconnect` copies it).
        // - All callback function pointers have matching C ABI signatures and catch all panics.
        // - `ctx_ptr` remains valid until after `openconnect_vpninfo_free` in `Drop`.
        let raw_vpninfo = unsafe {
            openconnect_vpninfo_new(
                useragent.as_ptr(),
                Some(crate::cert::PeerCerts::validate_peer_cert),
                None,
                Some(crate::form::FormManager::process_auth_form_cb),
                Some(helper_format_vargs),
                ctx_ptr.as_ptr().cast::<c_void>(),
            )
        };

        let Some(vpninfo) = NonNull::new(raw_vpninfo) else {
            // SAFETY: `openconnect_vpninfo_new` failed and retained no reference to `ctx_ptr`.
            unsafe {
                drop(Box::from_raw(ctx_ptr.as_ptr()));
            }
            return Err(OpenconnectError::CreateVpnInfoError);
        };

        // SAFETY: `ctx_ptr` is uniquely owned here before any C API that triggers callbacks is invoked.
        unsafe {
            ctx_ptr
                .as_ref()
                .vpninfo
                .store(vpninfo.as_ptr(), Ordering::Release);
        }

        Ok(Self {
            vpninfo,
            ctx_ptr,
            state,
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut openconnect_info {
        self.vpninfo.as_ptr()
    }
}

impl Drop for VpnInfoHandle {
    fn drop(&mut self) {
        // Invalidate the command pipe FD under lock BEFORE `openconnect_vpninfo_free` closes it,
        // preventing any concurrent `send_command` from writing into a closed/reused FD.
        if let Ok(mut fd_guard) = self.state.cmd_fd.lock() {
            *fd_guard = None;
        }

        // SAFETY: `self.vpninfo` was created by `openconnect_vpninfo_new` and is freed exactly once.
        unsafe {
            openconnect_vpninfo_free(self.vpninfo.as_ptr());
        }

        // SAFETY: `openconnect_vpninfo_free` has completed, so `libopenconnect` can no longer
        // invoke any callback referencing `self.ctx_ptr`. Reclaim the boxed context cleanly.
        unsafe {
            drop(Box::from_raw(self.ctx_ptr.as_ptr()));
        }

        tracing::debug!("VpnInfoHandle and CallbackContext dropped cleanly");
    }
}
