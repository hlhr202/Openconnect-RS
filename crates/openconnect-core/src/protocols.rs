use crate::{
    ffi::c_str_to_option_string,
    result::{OpenconnectError, OpenconnectResult},
};
use openconnect_sys::{
    oc_vpn_proto, openconnect_free_supported_protocols, openconnect_get_supported_protocols,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocol {
    pub name: String,
    pub pretty_name: String,
    pub description: String,
    pub flags: u32,
}

/// RAII guard ensuring `openconnect_free_supported_protocols` is always called.
struct ProtoListGuard(*mut oc_vpn_proto);

impl Drop for ProtoListGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is the base pointer allocated by `openconnect_get_supported_protocols`.
            unsafe {
                openconnect_free_supported_protocols(self.0);
            }
        }
    }
}

pub fn get_supported_protocols() -> OpenconnectResult<Vec<Protocol>> {
    let mut raw_protocols = std::ptr::null_mut::<oc_vpn_proto>();

    // SAFETY: Passing a valid writable out-pointer to `openconnect_get_supported_protocols`.
    let n = unsafe { openconnect_get_supported_protocols(&mut raw_protocols) };
    let _guard = ProtoListGuard(raw_protocols);

    if n < 0 {
        return Err(OpenconnectError::GetProtocolsError(n));
    }

    if n == 0 || raw_protocols.is_null() {
        return Ok(Vec::new());
    }

    let count = usize::try_from(n).map_err(|_| OpenconnectError::GetProtocolsError(n))?;

    // SAFETY: `openconnect_get_supported_protocols` returned `n > 0` and populated `raw_protocols`
    // with an array of at least `count` initialized `oc_vpn_proto` entries, kept alive by `_guard`.
    let slice = unsafe { std::slice::from_raw_parts(raw_protocols, count) };

    let mut protocols = Vec::with_capacity(count);
    for proto in slice {
        // SAFETY: Non-null `name` field in `oc_vpn_proto` is a valid static NUL-terminated C string.
        let name = unsafe { c_str_to_option_string(proto.name) };
        // SAFETY: Non-null `pretty_name` field in `oc_vpn_proto` is a valid static NUL-terminated C string.
        let pretty_name = unsafe { c_str_to_option_string(proto.pretty_name) }.unwrap_or_default();
        // SAFETY: Non-null `description` field in `oc_vpn_proto` is a valid static NUL-terminated C string.
        let description = unsafe { c_str_to_option_string(proto.description) }.unwrap_or_default();

        let Some(name) = name else {
            break;
        };

        protocols.push(Protocol {
            name,
            pretty_name,
            description,
            flags: proto.flags,
        });
    }

    Ok(protocols)
}

pub fn get_anyconnect_protocol() -> OpenconnectResult<Protocol> {
    get_supported_protocols()?
        .into_iter()
        .find(|p| p.name == "anyconnect")
        .ok_or_else(|| {
            OpenconnectError::OtherError("anyconnect protocol not supported".to_string())
        })
}
