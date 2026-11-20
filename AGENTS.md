# AGENTS.md — OpenConnect-RS Maintenance Guide

This repository is maintained with strict Rust memory-safety, FFI soundness, and automated regression standards.

## Workspace Layout

| Crate | Purpose | FFI / Safety Notes |
|---|---|---|
| `crates/openconnect-sys` | Raw C bindings (`libopenconnect`) + `c-src/helper.c` + `c-src/mock_openconnect.c` | Supports `--no-default-features --features mock-ffi` for offline/CI testing without native `libopenconnect.a`. Never write generated bindings into `src/` from `build.rs` (write to `OUT_DIR`). |
| `crates/openconnect-core` | Safe Rust wrapper (`VpnClient`, forms, certs, stats, signals, encrypted storage) | Zero `unsafe impl Sync`. All `libopenconnect` calls are serialized via `Mutex<VpnInfoHandle>`. All `extern "C"` callbacks MUST use `with_callback_context` / `ffi_catch_unwind`. |
| `crates/openconnect-oidc` | OIDC Device Flow & Authorization Code + PKCE (`openidconnect` 4.x) | TLS verification is enabled by default (`allow_insecure` must be explicitly passed). OIDC HTTP client disables redirects. |
| `crates/openconnect-cli` | CLI + detached background daemon (`__daemon` re-exec) + Unix domain socket IPC | Zero `libc::raise` / `libc::fork`. Socket lives in owner-only `0700` dir with `0600` mode and `peer_cred` UID check. |
| `crates/openconnect-gui` | Tauri v2 desktop app + React/HeroUI/Vite frontend | Strict CSP in `tauri.conf.json`, least-privilege `capabilities/default.json`, NO `window.eval` on untrusted URIs. |

## Non-Negotiable Safety Invariants

1. **Workspace Lints**: `unsafe_op_in_unsafe_fn = "deny"`, `clippy::undocumented_unsafe_blocks = "deny"`, `clippy::multiple_unsafe_ops_per_block = "warn"`. Every `unsafe` block must hold a single unsafe operation with a `// SAFETY:` justification.
2. **Ownership Topology (`openconnect-core`)**:
   - `VpnInfoHandle` exclusively owns both `NonNull<openconnect_info>` and `Box<CallbackContext>`.
   - `VpnInfoHandle::drop` MUST: (1) clear `SharedState::cmd_fd` under lock, (2) call `openconnect_vpninfo_free`, and (3) only afterwards reclaim `Box::from_raw(ctx_ptr)`.
   - Never use `Weak::into_raw` or mutate behind `Arc` without `UnsafeCell` / `Mutex`.
   - Never expose safe trait impls (like `From<&oc_ip_info>`) that dereference raw C pointers.
3. **FFI Callback Rules**:
   - Never allow a Rust panic to unwind across `extern "C"` or `extern "system"`. Wrap every callback body in `with_callback_context` or `ffi_catch_unwind`.
   - Always null-check every C pointer before `CStr::from_ptr` or dereferencing; use `c_str_to_option_string`.
   - Never assign borrowed pointers to `oc_form_opt._value`; always use `openconnect_set_option_value` so C duplicates the string cleanly.
4. **C Helper (`c-src/helper.c`)**:
   - Guard `len > 0` before indexing `buf[len - 1]`.
   - Access the global progress callback atomically.
   - Always verify C changes under `gcc -fsanitize=address,undefined`.

## Standard Verification Commands

Run these before completing any change:

```bash
# 1. Backend unit + promoted FFI safety regression tests (no native libopenconnect needed)
cargo test -p openconnect-sys --no-default-features --features mock-ffi
cargo test -p openconnect-core --no-default-features --features mock-ffi
cargo test -p openconnect-oidc
cargo test -p openconnect-cli --no-default-features --features mock-ffi

# 2. Strict lint & formatting checks
cargo fmt --all -- --check
cargo clippy -p openconnect-core --no-default-features --features mock-ffi --all-targets -- -D warnings
cargo clippy -p openconnect-oidc --all-targets -- -D warnings
cargo clippy -p openconnect-cli --no-default-features --features mock-ffi --all-targets -- -D warnings

# 3. C helper ASan + UBSan check
gcc -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer \
  -Icrates/openconnect-sys/c-src \
  crates/openconnect-sys/c-src/helper.c \
  fixtures/safety-audit/01_helper_c_stack_underflow.c \
  -o /tmp/test_helper_asan && /tmp/test_helper_asan

# 4. GUI frontend typecheck & production bundle
cd crates/openconnect-gui && pnpm build
```
