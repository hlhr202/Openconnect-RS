//! Peer certificate trust handling.
//!
//! `libopenconnect` only invokes the peer-certificate callback when the server certificate
//! fails regular CA-chain / hostname verification. In that case this module implements a
//! trust-on-first-use (TOFU) model mirroring the original Qt `openconnect-gui`:
//!
//! 1. Certificates already accepted during this client session are accepted again
//!    (e.g. for the CSTP connection or reconnects after the cookie was obtained).
//! 2. If the [`crate::config::Entrypoint::server_cert`] pin matches, the certificate is accepted.
//! 3. If `accept_insecure_cert` is enabled, any certificate is accepted (advanced, unsafe).
//! 4. Otherwise the user-supplied [`crate::events::EventHandlers::with_handle_peer_cert_verify`]
//!    handler is asked, with [`PeerCertTrust::Unknown`] (first connection) or
//!    [`PeerCertTrust::Mismatch`] (the stored pin no longer matches).
//!
//! Without a handler, unknown or mismatching certificates are rejected (fail-closed).

use crate::ffi::{CallbackContext, c_str_to_option_string, with_callback_context};
use base64::Engine;
use openconnect_sys::{
    openconnect_check_peer_cert_hash, openconnect_free_cert_info, openconnect_get_hostname,
    openconnect_get_peer_cert_details, openconnect_get_peer_cert_hash, openconnect_get_port,
    openconnect_info,
};
use std::{ffi::CString, sync::Mutex};

/// Prefix of the public-key pin format produced by `openconnect_get_peer_cert_hash`.
pub const PIN_SHA256_PREFIX: &str = "pin-sha256:";

/// Length in bytes of a SHA-256 digest.
const SHA256_LEN: usize = 32;

/// Why the user is asked to decide whether to trust a peer certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerCertTrust {
    /// No fingerprint is stored for this server yet (first connection).
    Unknown,

    /// A fingerprint is stored for this server, but the server now presents a different key.
    /// This may be a legitimate key rotation or a man-in-the-middle attack.
    Mismatch {
        /// The previously stored (trusted) fingerprint.
        stored: String,
    },
}

/// Information about an unverifiable peer certificate, passed to the
/// [`crate::events::EventHandlers::with_handle_peer_cert_verify`] handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerCertPrompt {
    /// Hostname of the VPN server, if known.
    pub host: Option<String>,

    /// Port of the VPN server.
    pub port: i32,

    /// Public-key fingerprint of the presented certificate (`pin-sha256:<base64>`).
    /// Persist this value as the server's trusted fingerprint when the user accepts.
    pub fingerprint: String,

    /// Human-readable reason reported by `libopenconnect` for the verification failure.
    pub reason: String,

    /// Human-readable certificate details (subject, issuer, validity...), if available.
    pub details: Option<String>,

    /// Whether this server is unknown or its stored fingerprint mismatches.
    pub trust: PeerCertTrust,
}

/// Validate and canonicalize a server certificate pin.
///
/// Only full-length `pin-sha256:<base64 SHA-256 of SubjectPublicKeyInfo>` pins are accepted.
/// `openconnect_check_peer_cert_hash` performs *prefix* matching (minimum 4 characters), so
/// accepting truncated pins would make them trivially collidable.
pub fn normalize_server_cert(pin: &str) -> Result<String, String> {
    let pin = pin.trim();
    let Some(encoded) = pin.strip_prefix(PIN_SHA256_PREFIX) else {
        return Err(format!(
            "certificate fingerprint must start with '{PIN_SHA256_PREFIX}'"
        ));
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("certificate fingerprint is not valid base64: {e}"))?;
    if decoded.len() != SHA256_LEN {
        return Err(format!(
            "certificate fingerprint must encode a {SHA256_LEN}-byte SHA-256 digest, got {} bytes",
            decoded.len()
        ));
    }
    Ok(format!(
        "{PIN_SHA256_PREFIX}{}",
        base64::engine::general_purpose::STANDARD.encode(decoded)
    ))
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AcceptedCert {
    pub fingerprint: String,
    pub host: Option<String>,
    pub port: i32,
}

#[derive(Debug, Default)]
pub(crate) struct PeerCerts {
    pub accepted_certs: Mutex<Vec<AcceptedCert>>,
}

/// Returns `true` only if `pin` is a full-length pin and `libopenconnect` confirms it matches.
fn peer_cert_matches(vpninfo_ptr: *mut openconnect_info, pin: &str) -> bool {
    let Ok(pin) = normalize_server_cert(pin) else {
        return false;
    };
    let Ok(pin_cstr) = CString::new(pin) else {
        return false;
    };
    // SAFETY: `vpninfo_ptr` is valid for this synchronous callback and `pin_cstr` outlives the call.
    let err = unsafe { openconnect_check_peer_cert_hash(vpninfo_ptr, pin_cstr.as_ptr()) };
    if err < 0 {
        tracing::warn!("Could not check peer certificate hash (error {})", err);
    }
    err == 0
}

/// Fetch human-readable certificate details, freeing the C allocation afterwards.
fn peer_cert_details(vpninfo_ptr: *mut openconnect_info) -> Option<String> {
    // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
    let raw = unsafe { openconnect_get_peer_cert_details(vpninfo_ptr) };
    if raw.is_null() {
        return None;
    }
    // SAFETY: `raw` is a non-null, NUL-terminated string allocated by `libopenconnect`.
    let details = unsafe { c_str_to_option_string(raw) };
    // SAFETY: `raw` was returned by `openconnect_get_peer_cert_details` and is freed exactly once.
    unsafe { openconnect_free_cert_info(vpninfo_ptr, raw.cast()) };
    details
}

impl PeerCerts {
    pub(crate) extern "C" fn validate_peer_cert(
        privdata: *mut ::std::os::raw::c_void,
        reason: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int {
        let body = |ctx: &CallbackContext| -> ::std::os::raw::c_int {
            let Some(vpninfo) = ctx.vpninfo_ptr() else {
                return 1;
            };
            let vpninfo_ptr = vpninfo.as_ptr();

            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let raw_host = unsafe { openconnect_get_hostname(vpninfo_ptr) };
            // SAFETY: `openconnect_get_hostname` returns NULL or a valid C string.
            let host = unsafe { c_str_to_option_string(raw_host) };
            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let port = unsafe { openconnect_get_port(vpninfo_ptr) };

            // 1. Certificates already accepted during this session.
            if let Ok(accepted_certs) = ctx.state.peer_certs.accepted_certs.lock() {
                let session_match = accepted_certs.iter().rev().any(|cert| {
                    (host.is_none() || cert.host == host)
                        && (port == 0 || cert.port == port)
                        && peer_cert_matches(vpninfo_ptr, &cert.fingerprint)
                });
                if session_match {
                    return 0;
                }
            }

            // 2. Persisted per-server pin.
            let trust = match ctx.state.stored_server_cert() {
                Some(stored) if peer_cert_matches(vpninfo_ptr, &stored) => {
                    tracing::debug!("Peer certificate matches stored fingerprint");
                    return 0;
                }
                Some(stored) => {
                    tracing::warn!("Peer certificate does not match the stored fingerprint");
                    PeerCertTrust::Mismatch { stored }
                }
                None => PeerCertTrust::Unknown,
            };

            // SAFETY: `vpninfo_ptr` is valid for the duration of this synchronous callback.
            let raw_hash = unsafe { openconnect_get_peer_cert_hash(vpninfo_ptr) };
            // SAFETY: `openconnect_get_peer_cert_hash` returns NULL or a valid C string.
            let Some(fingerprint) = (unsafe { c_str_to_option_string(raw_hash) }) else {
                tracing::error!("Peer certificate hash unavailable; rejecting certificate");
                return 1;
            };

            // SAFETY: `reason` is NULL or a valid C string provided by `libopenconnect`.
            let reason = unsafe { c_str_to_option_string(reason) }
                .unwrap_or_else(|| "unknown verification failure".to_string());

            let prompt = PeerCertPrompt {
                host: host.clone(),
                port,
                fingerprint: fingerprint.clone(),
                reason,
                details: peer_cert_details(vpninfo_ptr),
                trust,
            };

            // 3 + 4. `accept_insecure_cert` or interactive user decision.
            if ctx.state.handle_unverified_cert(&prompt) {
                if let Ok(mut accepted_certs) = ctx.state.peer_certs.accepted_certs.lock() {
                    accepted_certs.push(AcceptedCert {
                        fingerprint,
                        host,
                        port,
                    });
                }
                tracing::debug!("Peer certificate accepted");
                0
            } else {
                tracing::debug!("Peer certificate rejected");
                1
            }
        };

        // Fail-closed: return `1` (reject certificate) if privdata is null or a panic occurs.
        // SAFETY: Invoked synchronously by `libopenconnect` while `VpnInfoHandle` is alive.
        unsafe { with_callback_context(privdata, 1, body) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "pin-sha256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";

    #[test]
    fn normalize_accepts_full_pin_and_trims() {
        assert_eq!(
            normalize_server_cert(&format!("  {VALID}\n")).unwrap(),
            VALID
        );
    }

    #[test]
    fn normalize_rejects_truncated_or_malformed_pins() {
        assert!(normalize_server_cert("pin-sha256:47DE").is_err());
        assert!(normalize_server_cert("pin-sha256:").is_err());
        assert!(normalize_server_cert("sha256:abcd").is_err());
        assert!(normalize_server_cert("47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=").is_err());
        assert!(normalize_server_cert("pin-sha256:not base64!!").is_err());
    }
}
