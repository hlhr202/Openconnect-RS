use openconnect_sys::{oc_vpn_proto, openconnect_get_supported_protocols};

#[derive(Debug, Clone)]
pub struct Protocol {
    pub name: String,
    pub pretty_name: String,
    pub description: String,
    pub flags: u32,
}

pub fn get_supported_protocols() -> Vec<Protocol> {
    let mut raw_protocols = std::ptr::null_mut::<oc_vpn_proto>();
    let mut protocols: Vec<Protocol> = vec![];
    unsafe {
        let n = openconnect_get_supported_protocols(&mut raw_protocols);
        if n < 0 {
            panic!("openconnect_get_supported_protocols failed");
        }
        while !raw_protocols.is_null() && !(*raw_protocols).name.is_null() {
            let name = std::ffi::CStr::from_ptr((*raw_protocols).name)
                .to_string_lossy()
                .to_string();
            let pretty_name = std::ffi::CStr::from_ptr((*raw_protocols).pretty_name)
                .to_string_lossy()
                .to_string();
            let description = std::ffi::CStr::from_ptr((*raw_protocols).description)
                .to_string_lossy()
                .to_string();
            let flags = (*raw_protocols).flags;
            protocols.push(Protocol {
                name,
                pretty_name,
                description,
                flags,
            });
            raw_protocols = raw_protocols.offset(1);
        }
    }
    protocols
}

// TODO: temp solution
pub fn get_anyconnect_protocol() -> Protocol {
    get_supported_protocols()
        .iter()
        .find(|p| p.name == "anyconnect")
        .expect("anyconnect protocol not found")
        .clone()
}
