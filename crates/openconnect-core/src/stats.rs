use crate::VpnClient;
use openconnect_sys::{oc_stats, openconnect_get_dtls_cipher};

#[derive(Debug)]
pub struct Stats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_pkts: u64,
    pub tx_pkts: u64,
}

pub(crate) extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, _stats: *const oc_stats) {
    println!("stats_fn");
    let client = VpnClient::from_c_void(privdata);
    unsafe {
        if !client.is_null() {
            let dlts = {
                let cipher = openconnect_get_dtls_cipher((*client).vpninfo);
                if !cipher.is_null() {
                    Some(
                        std::ffi::CStr::from_ptr(cipher)
                            .to_str()
                            .unwrap()
                            .to_string(),
                    )
                } else {
                    None
                }
            };

            let stats: Option<Stats> = if !_stats.is_null() {
                let stats = *_stats;
                Some(Stats {
                    rx_bytes: stats.rx_bytes,
                    tx_bytes: stats.tx_bytes,
                    rx_pkts: stats.rx_pkts,
                    tx_pkts: stats.tx_pkts,
                })
            } else {
                None
            };

            (*client).handle_stats((dlts, stats))
        }
    }
}
