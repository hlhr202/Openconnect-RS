use crate::ffi::{CallbackContext, c_str_to_option_string, with_callback_context};
use openconnect_sys::{oc_stats, openconnect_get_dtls_cipher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_pkts: u64,
    pub tx_pkts: u64,
}

pub(crate) extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, stats: *const oc_stats) {
    let body = |ctx: &CallbackContext| {
        let dtls = ctx.vpninfo_ptr().and_then(|vpninfo| {
            // SAFETY: `vpninfo` is valid for the duration of this synchronous callback.
            let cipher = unsafe { openconnect_get_dtls_cipher(vpninfo.as_ptr()) };
            // SAFETY: `openconnect_get_dtls_cipher` returns NULL or a valid C string.
            unsafe { c_str_to_option_string(cipher) }
        });

        // SAFETY: `libopenconnect` passes either NULL or a valid aligned `oc_stats` pointer.
        let stats = unsafe { stats.as_ref() }.map(|s| Stats {
            rx_bytes: s.rx_bytes,
            tx_bytes: s.tx_bytes,
            rx_pkts: s.rx_pkts,
            tx_pkts: s.tx_pkts,
        });

        ctx.state.handle_stats((dtls, stats));
    };

    // SAFETY: Invoked synchronously by `libopenconnect` while `VpnInfoHandle` is alive.
    unsafe { with_callback_context(privdata, (), body) };
}
