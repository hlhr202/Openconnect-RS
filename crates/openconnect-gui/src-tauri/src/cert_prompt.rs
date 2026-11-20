//! Bridges `libopenconnect`'s synchronous peer-certificate callback to an asynchronous
//! frontend dialog (trust-on-first-use, mirroring the original Qt `openconnect-gui`).

use openconnect_core::cert::{PeerCertPrompt, PeerCertTrust};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::Duration,
};

/// How long the connection waits for the user before rejecting the certificate.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(300);

/// Payload of the `vpnCertPrompt` event sent to the frontend.
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CertPromptPayload {
    pub id: u64,
    pub server_name: String,
    pub host: Option<String>,
    pub port: i32,
    pub fingerprint: String,
    /// Previously trusted fingerprint when the server key changed; `None` for unknown servers.
    pub stored_fingerprint: Option<String>,
    pub reason: String,
    pub details: Option<String>,
}

struct PendingPrompt {
    server_name: String,
    fingerprint: String,
    responder: SyncSender<bool>,
}

/// What the frontend's answer resolved to.
pub struct ResolvedPrompt {
    pub server_name: String,
    pub fingerprint: String,
    responder: SyncSender<bool>,
}

impl ResolvedPrompt {
    /// Unblock the waiting connection thread with the user's decision.
    pub fn respond(self, accept: bool) {
        let _ = self.responder.send(accept);
    }
}

#[derive(Default)]
pub struct CertPromptBroker {
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, PendingPrompt>>,
}

impl CertPromptBroker {
    /// Register a prompt and block the calling (connection) thread until the frontend answers
    /// or [`PROMPT_TIMEOUT`] elapses. `emit` must deliver the payload to the frontend.
    ///
    /// Must only be called from a blocking thread (never from an async task).
    pub fn ask_blocking(
        &self,
        server_name: &str,
        prompt: &PeerCertPrompt,
        emit: impl FnOnce(CertPromptPayload),
    ) -> bool {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = sync_channel::<bool>(1);

        match self.pending.lock() {
            Ok(mut pending) => {
                pending.insert(
                    id,
                    PendingPrompt {
                        server_name: server_name.to_string(),
                        fingerprint: prompt.fingerprint.clone(),
                        responder: tx,
                    },
                );
            }
            Err(_) => return false,
        }

        emit(CertPromptPayload {
            id,
            server_name: server_name.to_string(),
            host: prompt.host.clone(),
            port: prompt.port,
            fingerprint: prompt.fingerprint.clone(),
            stored_fingerprint: match &prompt.trust {
                PeerCertTrust::Unknown => None,
                PeerCertTrust::Mismatch { stored } => Some(stored.clone()),
            },
            reason: prompt.reason.clone(),
            details: prompt.details.clone(),
        });

        let accepted = rx.recv_timeout(PROMPT_TIMEOUT).unwrap_or(false);
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&id);
        }
        accepted
    }

    /// Take the pending prompt `id` so the caller can persist the fingerprint before responding.
    pub fn resolve(&self, id: u64) -> Option<ResolvedPrompt> {
        let prompt = self.pending.lock().ok()?.remove(&id)?;
        Some(ResolvedPrompt {
            server_name: prompt.server_name,
            fingerprint: prompt.fingerprint,
            responder: prompt.responder,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn prompt() -> PeerCertPrompt {
        PeerCertPrompt {
            host: Some("vpn.example.test".into()),
            port: 443,
            fingerprint: "pin-sha256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=".into(),
            reason: "signer not found".into(),
            details: None,
            trust: PeerCertTrust::Unknown,
        }
    }

    #[test]
    fn answer_unblocks_waiting_thread() {
        for accept in [true, false] {
            let broker = Arc::new(CertPromptBroker::default());
            let (id_tx, id_rx) = std::sync::mpsc::channel();
            let waiter = {
                let broker = Arc::clone(&broker);
                std::thread::spawn(move || {
                    broker.ask_blocking("corp", &prompt(), |payload| {
                        id_tx.send(payload.id).unwrap();
                    })
                })
            };
            let id = id_rx.recv().unwrap();
            let resolved = broker.resolve(id).expect("pending prompt");
            assert_eq!(resolved.server_name, "corp");
            resolved.respond(accept);
            assert_eq!(waiter.join().unwrap(), accept);
            assert!(broker.resolve(id).is_none(), "prompt is consumed");
        }
    }
}
