use openconnect_sys::oc_ip_info;
use std::ffi::CStr;

// TODO: Implement SplitInclude
// pub struct SplitInclude {
//     pub route: String,
//     pub next: Arc<SplitInclude>,
// }

#[derive(serde::Serialize, serde::Deserialize)]
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
    // pub split_dns: SplitInclude,
    // pub split_includes: SplitInclude,
    // pub split_excludes: SplitInclude,
    pub gateway_addr: Option<String>,
}

unsafe fn raw_to_string(raw: *const i8) -> Option<String> {
    if raw.is_null() {
        None
    } else {
        Some(CStr::from_ptr(raw).to_string_lossy().to_string())
    }
}

impl From<&oc_ip_info> for IpInfo {
    fn from(value: &oc_ip_info) -> Self {
        unsafe {
            // let value = value.as_ref();
            Self {
                addr: raw_to_string(value.addr),
                netmask: raw_to_string(value.netmask),
                addr6: raw_to_string(value.addr6),
                netmask6: raw_to_string(value.netmask6),
                dns: [
                    raw_to_string(value.dns[0]),
                    raw_to_string(value.dns[1]),
                    raw_to_string(value.dns[2]),
                ],
                nbns: [
                    raw_to_string(value.nbns[0]),
                    raw_to_string(value.nbns[1]),
                    raw_to_string(value.nbns[2]),
                ],
                domain: raw_to_string(value.domain),
                proxy_pac: raw_to_string(value.proxy_pac),
                mtu: value.mtu,
                gateway_addr: raw_to_string(value.gateway_addr),
            }
        }
    }
}
