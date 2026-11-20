//! Interactive trust-on-first-use prompt for unverifiable server certificates,
//! mirroring the behaviour of the original `openconnect` CLI and `openconnect-gui`.

use colored::Colorize;
use openconnect_core::cert::{PeerCertPrompt, PeerCertTrust};
use std::io::{BufRead, IsTerminal, Write};

/// Serializable description of a server certificate that could not be verified.
/// Sent from the daemon to the client so the user can decide interactively.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UntrustedCert {
    pub host: Option<String>,
    pub port: i32,
    pub fingerprint: String,
    pub reason: String,
    pub details: Option<String>,
    /// Previously trusted fingerprint, if the stored one no longer matches.
    pub stored_fingerprint: Option<String>,
}

impl From<&PeerCertPrompt> for UntrustedCert {
    fn from(prompt: &PeerCertPrompt) -> Self {
        Self {
            host: prompt.host.clone(),
            port: prompt.port,
            fingerprint: prompt.fingerprint.clone(),
            reason: prompt.reason.clone(),
            details: prompt.details.clone(),
            stored_fingerprint: match &prompt.trust {
                PeerCertTrust::Unknown => None,
                PeerCertTrust::Mismatch { stored } => Some(stored.clone()),
            },
        }
    }
}

/// Ask the user on the terminal whether to trust `cert`. Returns `false` when stdin is not
/// an interactive terminal (fail-closed).
pub fn ask_trust_cert(cert: &UntrustedCert) -> bool {
    let host = cert.host.as_deref().unwrap_or("<unknown>");
    let mut err = std::io::stderr().lock();

    let _ = writeln!(err);
    match &cert.stored_fingerprint {
        None => {
            let _ = writeln!(
                err,
                "{}",
                format!("Certificate from VPN server \"{host}\" cannot be verified by a trusted authority.")
                    .yellow()
                    .bold()
            );
            let _ = writeln!(
                err,
                "There is no guarantee that the server is the computer you think it is."
            );
        }
        Some(stored) => {
            let _ = writeln!(
                err,
                "{}",
                format!("WARNING: the certificate key of VPN server \"{host}\" has CHANGED!")
                    .red()
                    .bold()
            );
            let _ = writeln!(
                err,
                "It may be that the server has multiple keys, the administrator replaced it, \
                 or you are (or were in the past) under attack."
            );
            let _ = writeln!(err, "Previously trusted: {stored}");
        }
    }
    let _ = writeln!(err, "Reason:      {}", cert.reason);
    let _ = writeln!(err, "Host:        {host}:{}", cert.port);
    let _ = writeln!(err, "Fingerprint: {}", cert.fingerprint.bold());
    let _ = writeln!(
        err,
        "Only accept if you validated this fingerprint through another trusted channel."
    );

    if !std::io::stdin().is_terminal() {
        let _ = writeln!(
            err,
            "{}",
            "Not running interactively; rejecting certificate.".red()
        );
        return false;
    }

    let stdin = std::io::stdin();
    loop {
        let _ = write!(
            err,
            "Enter 'yes' to trust and remember, 'no' to abort; anything else to view details: "
        );
        let _ = err.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => return false,
            Ok(_) => {}
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "yes" => return true,
            "no" => return false,
            _ => {
                let _ = writeln!(
                    err,
                    "\n{}\n",
                    cert.details
                        .as_deref()
                        .unwrap_or("No certificate details available.")
                );
            }
        }
    }
}
