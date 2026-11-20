#![cfg(feature = "mock-ffi")]

//! Promoted safety regression suite proving all pointer, FFI, memory-safety,
//! and soundness bugs audited in v0.1.x are permanently fixed.

use openconnect_core::{
    Connectable, Status, VpnClient,
    cert::{PeerCertPrompt, PeerCertTrust},
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    protocols::{get_anyconnect_protocol, get_supported_protocols},
};
use openconnect_sys::mock;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// Serialize mock-FFI tests that assert exact global C allocation counters.
static COUNTER_LOCK: Mutex<()> = Mutex::new(());

fn default_client(handlers: EventHandlers) -> Arc<VpnClient> {
    let config = ConfigBuilder::default()
        .loglevel(LogLevel::Debug)
        .build()
        .expect("config build");
    VpnClient::new(config, handlers).expect("VpnClient::new")
}

#[test]
fn regression_01_helper_c_empty_format_and_unknown_log_level() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let client = default_client(EventHandlers::default());
    client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid for the duration of `with_mock_vpninfo`.
        // Previously caused stack-buffer-underflow in `helper.c:20` (`buf[len - 1]`).
        unsafe { mock::mock_oc_trigger_progress_empty(vpninfo) };
        // SAFETY: `vpninfo` is valid for the duration of `with_mock_vpninfo`.
        // Previously caused `unreachable!()` panic across FFI in `log.rs:57`.
        unsafe { mock::mock_oc_trigger_progress_unknown_level(vpninfo) };
    });
}

#[test]
fn regression_02_vpnclient_drop_frees_vpninfo_and_context_without_leak() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by `COUNTER_LOCK`.
    let before = unsafe { mock::mock_oc_live_vpninfo_count() };

    {
        let client = default_client(EventHandlers::default());
        // SAFETY: Reading atomic C counter.
        let during = unsafe { mock::mock_oc_live_vpninfo_count() };
        assert_eq!(during, before + 1);
        drop(client);
    }

    // SAFETY: Reading atomic C counter after Drop.
    let after = unsafe { mock::mock_oc_live_vpninfo_count() };
    assert_eq!(
        after, before,
        "VpnClient::drop must free openconnect_info and CallbackContext cleanly"
    );
}

#[test]
fn regression_03_event_handlers_sync_and_concurrent_state_updates() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let seen = Arc::new(AtomicUsize::new(0));
    let seen_cb = Arc::clone(&seen);

    // Note: `!Sync` types like `RefCell` are now statically rejected at compile time
    // because `EventHandlers` requires `F: Send + Sync + 'static`.
    let handlers = EventHandlers::new()
        .with_handle_connection_state_change(move |_status| {
            seen_cb.fetch_add(1, Ordering::SeqCst);
        })
        .with_handle_peer_cert_verify(|_prompt| {
            // Even a panicking user callback must be caught at the FFI boundary (fail-closed)!
            panic!("simulated user callback panic");
        });

    let client = default_client(handlers);
    assert!(seen.load(Ordering::SeqCst) >= 1);

    // Trigger validate_peer_cert when user callback panics -> must NOT unwind across FFI
    // and must fail-closed by returning `1` (reject).
    let cert_result = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_validate_peer_cert(vpninfo, c"self-signed".as_ptr()) }
    });
    assert_eq!(
        cert_result, 1,
        "Panicking cert callback must be caught and reject certificate (fail-closed)"
    );
}

#[test]
fn regression_04_get_info_passes_valid_out_pointer() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let client = default_client(EventHandlers::default());
    let info = client
        .get_info()
        .expect("get_info should succeed")
        .expect("get_info must return Some(IpInfo), not silently None");
    assert_eq!(info.addr.as_deref(), Some("10.8.0.2"));
    assert_eq!(info.netmask.as_deref(), Some("255.255.255.0"));
    assert_eq!(info.mtu, 1400);
    assert_eq!(info.gateway_addr.as_deref(), Some("198.51.100.1"));
}

#[test]
fn regression_05_nullable_c_strings_never_segfault() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let client = default_client(EventHandlers::default());
    client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe {
            mock::mock_oc_set_return_nulls(vpninfo, 1);
        }
    });

    assert_eq!(client.get_cookie(), None);
    assert_eq!(client.get_hostname(), None);
    assert_eq!(client.get_peer_cert_hash(), None);
    assert_eq!(client.get_dtls_cipher(), None);
    assert_eq!(client.get_info().unwrap(), None);

    // Also test stats_fn & setup_tun callbacks when C returns NULL strings / NULL stats.
    client.set_stats_handler();
    client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_stats(vpninfo, 0) };
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_stats(vpninfo, 1) };
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_setup_tun(vpninfo) };
    });
}

#[test]
fn regression_06_form_callbacks_no_infinite_loop_no_double_free_no_panic() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let client = default_client(EventHandlers::default());
    let entrypoint = EntrypointBuilder::new()
        .name("corp")
        .server("https://vpn.example.test")
        .username("alice")
        .password("s3cr3t")
        .build()
        .expect("entrypoint build");
    client
        .connect_for_cookie(entrypoint)
        .expect("connect_for_cookie");

    // 6a. OC_FORM_OPT_IGNORE must terminate cleanly instead of infinite-looping.
    let rc_ignore = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_auth_form_ignored_opt(vpninfo) }
    });
    assert_eq!(rc_ignore, 0);

    // 6b. Exact & partial OC_FORM_OPT_SELECT matches use openconnect_set_option_value (no double-free!)
    client.save_mock_form_field("main", "group_list", Some("Corp"));
    let rc_select = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe {
            mock::mock_oc_trigger_auth_form_select(
                vpninfo,
                c"corp_id".as_ptr(),
                c"Corp".as_ptr(),
                c"group_list".as_ptr(),
            )
        }
    });
    assert_eq!(rc_select, 0);

    // 6c. Saved input longer than choice label & multi-byte UTF-8 must NOT slice-panic across FFI.
    client.save_mock_form_field("main", "short_group", Some("VERY_LONG_SAVED_CHOICE_LABEL"));
    let rc_long = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe {
            mock::mock_oc_trigger_auth_form_select(
                vpninfo,
                c"s".as_ptr(),
                c"AB".as_ptr(),
                c"short_group".as_ptr(),
            )
        }
    });
    assert_eq!(rc_long, 0);

    client.save_mock_form_field("main", "utf8_group", Some("a"));
    let rc_utf8 = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe {
            mock::mock_oc_trigger_auth_form_select(
                vpninfo,
                c"u".as_ptr(),
                c"中文组".as_ptr(),
                c"utf8_group".as_ptr(),
            )
        }
    });
    assert_eq!(rc_utf8, 0);

    // 6d. Unknown text field must NOT hit `todo!()` panic across FFI.
    let rc_unknown_text = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe {
            mock::mock_oc_trigger_auth_form_text_and_password(
                vpninfo,
                c"unexpected_otp_field".as_ptr(),
            )
        }
    });
    assert_eq!(rc_unknown_text, 0);

    // 6e. NULL form / NULL auth_id / NULL opt->name must return errors/skip cleanly without SIGSEGV.
    let rc_nulls = client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_auth_form_null_cases(vpninfo) }
    });
    assert_eq!(rc_nulls, 0);
}

#[test]
fn regression_07_supported_protocols_zero_c_heap_leak() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // SAFETY: Serialized by `COUNTER_LOCK`.
    let before = unsafe { mock::mock_oc_live_proto_alloc_count() };

    for _ in 0..50 {
        let protos = get_supported_protocols().expect("get_supported_protocols");
        assert_eq!(protos.len(), 2);
        assert_eq!(protos[0].name, "anyconnect");
        assert_eq!(protos[1].name, "gp");
        assert_eq!(protos[1].description, ""); // NULL description safely becomes ""
    }

    let anyconnect = get_anyconnect_protocol().expect("get_anyconnect_protocol");
    assert_eq!(anyconnect.name, "anyconnect");

    // SAFETY: Reading atomic C allocation balance counter.
    let after = unsafe { mock::mock_oc_live_proto_alloc_count() };
    assert_eq!(
        after, before,
        "All oc_vpn_proto arrays must be freed via openconnect_free_supported_protocols"
    );
}

#[test]
fn regression_08_end_to_end_run_loop_concurrent_queries_and_graceful_disconnect() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let client = default_client(EventHandlers::default());
    let entrypoint = EntrypointBuilder::new()
        .name("prod-vpn")
        .server("https://vpn.example.test")
        .cookie("mock-cookie=abc")
        .build()
        .expect("entrypoint");

    client.init_connection(entrypoint).expect("init_connection");
    assert_eq!(client.get_status(), Status::Connected);

    let loop_client = Arc::clone(&client);
    let worker = std::thread::spawn(move || loop_client.run_loop());

    // Give worker a moment to enter `openconnect_mainloop` (holding the C handle lock).
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Concurrent reads from another thread must succeed cleanly via cached snapshots without racing on C `vpninfo`!
    assert_eq!(
        client.get_hostname().as_deref(),
        Some("https://vpn.example.test")
    );
    assert_eq!(
        client.get_info().unwrap().and_then(|i| i.addr),
        Some("10.8.0.2".to_string())
    );
    assert_eq!(client.get_server_name().as_deref(), Some("prod-vpn"));

    // Disconnect writes OC_CMD_CANCEL to the real pipe, waking up `mock_openconnect_mainloop`!
    client.disconnect();
    worker.join().expect("worker join").expect("run_loop ok");
    assert_eq!(client.get_status(), Status::Disconnected);
}

const MOCK_PIN: &str = "pin-sha256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";
const OTHER_PIN: &str = "pin-sha256:qUiQTy8PR5uPgZdpSzAYSw0u0cHNKh7A+4XSmaGSpEc=";

/// Client whose peer-cert handler records prompts and answers with `accept`.
fn tofu_client(
    accept: bool,
    server_cert: Option<&str>,
) -> (Arc<VpnClient>, Arc<Mutex<Vec<PeerCertPrompt>>>) {
    let prompts = Arc::new(Mutex::new(Vec::new()));
    let prompts_cb = Arc::clone(&prompts);
    let handlers = EventHandlers::new().with_handle_peer_cert_verify(move |prompt| {
        prompts_cb.lock().unwrap().push(prompt.clone());
        accept
    });
    let client = default_client(handlers);
    let mut builder = EntrypointBuilder::new();
    builder
        .name("tofu")
        .server("https://vpn.example.test")
        .cookie("mock-cookie=abc");
    if let Some(pin) = server_cert {
        builder.server_cert(pin);
    }
    client
        .connect_for_cookie(builder.build().expect("entrypoint"))
        .expect("connect_for_cookie");
    (client, prompts)
}

fn trigger_validate(client: &VpnClient) -> i32 {
    client.with_mock_vpninfo(|vpninfo| {
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`.
        unsafe { mock::mock_oc_trigger_validate_peer_cert(vpninfo, c"signer not found".as_ptr()) }
    })
}

#[test]
fn regression_09_peer_cert_trust_on_first_use() {
    let _guard = COUNTER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Unknown server, user rejects -> fail-closed with full prompt context.
    let (client, prompts) = tofu_client(false, None);
    assert_eq!(trigger_validate(&client), 1);
    {
        let prompts = prompts.lock().unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].trust, PeerCertTrust::Unknown);
        assert_eq!(prompts[0].fingerprint, MOCK_PIN);
        assert_eq!(prompts[0].reason, "signer not found");
        assert!(prompts[0].details.as_deref().unwrap_or("").contains("CN="));
    }
    drop(client);

    // Stored pin matches -> accepted silently.
    let (client, prompts) = tofu_client(false, Some(MOCK_PIN));
    assert_eq!(trigger_validate(&client), 0);
    assert!(prompts.lock().unwrap().is_empty());
    drop(client);

    // Stored pin mismatches -> user is warned; acceptance is cached for the session.
    let (client, prompts) = tofu_client(true, Some(OTHER_PIN));
    assert_eq!(trigger_validate(&client), 0);
    assert_eq!(trigger_validate(&client), 0);
    {
        let prompts = prompts.lock().unwrap();
        assert_eq!(prompts.len(), 1, "second handshake uses session cache");
        assert_eq!(
            prompts[0].trust,
            PeerCertTrust::Mismatch {
                stored: OTHER_PIN.to_string()
            }
        );
    }
    drop(client);

    // Truncated pins (accepted by libopenconnect's prefix matching) are never trusted.
    let (client, prompts) = tofu_client(false, Some(&MOCK_PIN[..20]));
    assert_eq!(trigger_validate(&client), 1);
    assert!(matches!(
        prompts.lock().unwrap()[0].trust,
        PeerCertTrust::Mismatch { .. }
    ));
    drop(client);

    // Server rotates its key mid-session -> cached acceptance no longer applies.
    let (client, prompts) = tofu_client(true, None);
    assert_eq!(trigger_validate(&client), 0);
    client.with_mock_vpninfo(|vpninfo| {
        let pin = std::ffi::CString::new(OTHER_PIN).unwrap();
        // SAFETY: `vpninfo` is valid inside `with_mock_vpninfo`; `pin` outlives the call.
        unsafe { mock::mock_oc_set_peer_cert_hash(vpninfo, pin.as_ptr()) }
    });
    assert_eq!(trigger_validate(&client), 0);
    assert_eq!(prompts.lock().unwrap().len(), 2);
    drop(client);

    // No handler registered -> unknown certificate rejected.
    let client = default_client(EventHandlers::default());
    let entrypoint = EntrypointBuilder::new()
        .server("https://vpn.example.test")
        .cookie("mock-cookie=abc")
        .build()
        .expect("entrypoint");
    client
        .connect_for_cookie(entrypoint)
        .expect("connect_for_cookie");
    assert_eq!(trigger_validate(&client), 1);
}
