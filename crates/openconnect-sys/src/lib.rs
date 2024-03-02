#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openconnect_version() {
        let version = unsafe {
            let raw_version = openconnect_get_version();
            std::ffi::CStr::from_ptr(raw_version).to_str().ok()
        };
        println!("OpenConnect version: {:?}", version.unwrap());
    }
}
