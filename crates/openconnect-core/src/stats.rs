use crate::VpnClient;
use openconnect_sys::oc_stats;

#[derive(Debug)]
pub struct Stats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_pkts: u64,
    pub tx_pkts: u64,
}

pub(crate) extern "C" fn stats_fn(privdata: *mut ::std::os::raw::c_void, stats: *const oc_stats) {
    println!("stats_fn");
    let client = unsafe { VpnClient::ref_from_raw(privdata) };
    let dlts = client.get_dlts_cipher();

    let stats: Option<Stats> = if !stats.is_null() {
        let stats = unsafe { &*stats };
        Some(Stats {
            rx_bytes: stats.rx_bytes,
            tx_bytes: stats.tx_bytes,
            rx_pkts: stats.rx_pkts,
            tx_pkts: stats.tx_pkts,
        })
    } else {
        None
    };

    client.handle_stats((dlts, stats));
}
