use crate::VpnClient;
use openconnect_sys::{
    oc_token_mode_t, oc_token_mode_t_OC_TOKEN_MODE_HOTP, oc_token_mode_t_OC_TOKEN_MODE_OIDC,
    oc_token_mode_t_OC_TOKEN_MODE_TOTP, openconnect_set_token_mode,
};
use std::ffi::CString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenMode {
    TOTP,
    HOTP,
    OIDC,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub mode: TokenMode,
    pub value: Option<String>,
}

impl From<TokenMode> for oc_token_mode_t {
    fn from(mode: TokenMode) -> Self {
        match mode {
            TokenMode::TOTP => oc_token_mode_t_OC_TOKEN_MODE_TOTP,
            TokenMode::HOTP => oc_token_mode_t_OC_TOKEN_MODE_HOTP,
            TokenMode::OIDC => oc_token_mode_t_OC_TOKEN_MODE_OIDC,
        }
    }
}

// TODO: Implement TokenMode::TOTP and TokenMode::HOTP
pub fn init_token(client: &VpnClient, token: Token) -> i32 {
    let token_str = token.value.map(|token| CString::new(token).unwrap());

    match token.mode {
        TokenMode::TOTP | TokenMode::HOTP => {
            // unimplemented
            -libc::ENOTSUP
        }
        TokenMode::OIDC => unsafe {
            openconnect_set_token_mode(
                client.vpninfo,
                token.mode.into(),
                token_str
                    .map(|token| token.as_ptr())
                    .unwrap_or(std::ptr::null()),
            )
        },
    }
}
