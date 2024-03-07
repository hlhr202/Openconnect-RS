use openconnect_sys::{oc_vpn_proto, openconnect_get_supported_protocols};

#[derive(Debug)]
pub struct Protocols {
    pub name: String,
    pub pretty_name: String,
    pub description: String,
    pub flags: u32,
}

pub fn get_supported_protocols() -> Vec<Protocols> {
    let mut raw_protocols = std::ptr::null_mut::<oc_vpn_proto>();
    let mut protocols: Vec<Protocols> = vec![];
    unsafe {
        let n = openconnect_get_supported_protocols(&mut raw_protocols);
        if n < 0 {
            panic!("openconnect_get_supported_protocols failed");
        }
        while !raw_protocols.is_null() && !(*raw_protocols).name.is_null() {
            let name = std::ffi::CStr::from_ptr((*raw_protocols).name)
                .to_str()
                .expect("protocol name is not valid")
                .to_string();
            let pretty_name = std::ffi::CStr::from_ptr((*raw_protocols).pretty_name)
                .to_str()
                .expect("protocol pretty_name is not valid")
                .to_string();
            let description = std::ffi::CStr::from_ptr((*raw_protocols).description)
                .to_str()
                .expect("protocol description is not valid")
                .to_string();
            let flags = (*raw_protocols).flags;
            protocols.push(Protocols {
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
