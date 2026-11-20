use crate::ffi::c_str_to_option_string;
use openconnect_sys::oc_ip_info;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IpInfo {
    pub addr: Option<String>,
    pub netmask: Option<String>,
    pub addr6: Option<String>,
    pub netmask6: Option<String>,
    pub dns: [Option<String>; 3],
    pub nbns: [Option<String>; 3],
    pub domain: Option<String>,
    pub proxy_pac: Option<String>,
    pub mtu: i32,
    pub gateway_addr: Option<String>,
}

impl IpInfo {
    /// Convert a raw `oc_ip_info` pointer returned by `openconnect_get_ip_info` into [`IpInfo`].
    ///
    /// # Safety
    /// If `ptr` is non-null, it must point to a valid, initialized `oc_ip_info` struct whose
    /// non-null string fields point to valid NUL-terminated C strings for the duration of this call.
    pub(crate) unsafe fn from_raw(ptr: *const oc_ip_info) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: Caller guarantees `ptr` is non-null, properly aligned, and points to a valid `oc_ip_info`.
        let value = unsafe { &*ptr };

        let read_str = |p| {
            // SAFETY: Caller guarantees every non-null field pointer in `oc_ip_info` is a valid C string.
            unsafe { c_str_to_option_string(p) }
        };

        Some(Self {
            addr: read_str(value.addr),
            netmask: read_str(value.netmask),
            addr6: read_str(value.addr6),
            netmask6: read_str(value.netmask6),
            dns: [
                read_str(value.dns[0]),
                read_str(value.dns[1]),
                read_str(value.dns[2]),
            ],
            nbns: [
                read_str(value.nbns[0]),
                read_str(value.nbns[1]),
                read_str(value.nbns[2]),
            ],
            domain: read_str(value.domain),
            proxy_pac: read_str(value.proxy_pac),
            mtu: value.mtu,
            gateway_addr: read_str(value.gateway_addr),
        })
    }
}
