# Safety Audit Fixtures & Post-Fix Regression Verification

This directory preserves the original minimal proof-of-concept fixtures (`01_`–`09_`) that reproduced the memory-safety, pointer, and FFI soundness bugs found in `v0.1.x`.

All fixes have been promoted into formal automated tests in:
- [`crates/openconnect-core/tests/ffi_safety_regression.rs`](../../crates/openconnect-core/tests/ffi_safety_regression.rs) (run via `cargo test -p openconnect-core --no-default-features --features mock-ffi`)
- [`crates/openconnect-sys/c-src/helper.c`](../../crates/openconnect-sys/c-src/helper.c) (verified with `gcc -fsanitize=address,undefined` against `01_helper_c_stack_underflow.c`)

## Running Post-Fix Verification

```bash
# 1. Verify fixed helper.c passes ASan + UBSan with zero errors
gcc -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer \
  -Icrates/openconnect-sys/c-src \
  crates/openconnect-sys/c-src/helper.c \
  fixtures/safety-audit/01_helper_c_stack_underflow.c \
  -o /tmp/test_helper_asan && /tmp/test_helper_asan

# 2. Run formal Rust + C mock-FFI regression tests
cargo test -p openconnect-core --no-default-features --features mock-ffi

# 3. Run Valgrind over the compiled ffi_safety_regression binary
TEST_BIN=$(cargo test -p openconnect-core --no-default-features --features mock-ffi --test ffi_safety_regression --no-run --message-format=json \
  | python3 -c "import sys,json; print([json.loads(l)['executable'] for l in sys.stdin if '\"executable\":' in l and 'ffi_safety_regression' in l][-1])")
valgrind --leak-check=full --show-leak-kinds=definite --error-exitcode=1 "$TEST_BIN" --test-threads=1
```
