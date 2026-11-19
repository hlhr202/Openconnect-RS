#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/.build"
mkdir -p "${BUILD_DIR}"

echo "============================================================"
echo " OpenConnect-RS Safety Audit Fixtures"
echo "============================================================"

# 1. helper.c stack-buffer-underflow (compiles REAL crates/openconnect-sys/c-src/helper.c with ASan)
echo ""
echo "--- [Fixture 01] Real helper.c stack-buffer-underflow (ASan + UBSan) ---"
gcc -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer \
    -I"${SCRIPT_DIR}/../../crates/openconnect-sys/c-src" \
    "${SCRIPT_DIR}/../../crates/openconnect-sys/c-src/helper.c" \
    "${SCRIPT_DIR}/01_helper_c_stack_underflow.c" \
    -o "${BUILD_DIR}/01_helper_c_stack_underflow"

set +e
ASAN_OUTPUT=$("${BUILD_DIR}/01_helper_c_stack_underflow" 2>&1)
ASAN_RC=$?
set -e
if [ "${ASAN_RC}" -ne 0 ] && echo "${ASAN_OUTPUT}" | grep -q "underflows this variable"; then
    echo "[01] PROVEN (UBSan + ASan stack-buffer-underflow on REAL helper.c, exit=${ASAN_RC}):"
    echo "${ASAN_OUTPUT}" | grep -E "runtime error:|ERROR: AddressSanitizer|READ of size|#0 |#1 |underflows this variable" | head -n 8
else
    echo "[01] UNEXPECTED: rc=${ASAN_RC}"
    echo "${ASAN_OUTPUT}"
    exit 1
fi

# Compile C mock library (normal + ASan)
gcc -c -g -O0 -fPIC "${SCRIPT_DIR}/mock_openconnect.c" -o "${BUILD_DIR}/mock_openconnect.o"
ar rcs "${BUILD_DIR}/libmock_openconnect.a" "${BUILD_DIR}/mock_openconnect.o"

# 2. Weak::into_raw leak + ref_from_raw UAF
echo ""
echo "--- [Fixture 02] Weak::into_raw leak & ref_from_raw UAF ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/02_weak_into_raw_uaf_and_leak.rs" -o "${BUILD_DIR}/02_weak_into_raw_uaf_and_leak"
"${BUILD_DIR}/02_weak_into_raw_uaf_and_leak"

# 3. Unsound Sync + !Sync EventHandlers
echo ""
echo "--- [Fixture 03] Unsound Sync for VpnClient with !Sync EventHandlers ---"
rustc --edition=2021 -O "${SCRIPT_DIR}/03_unsound_sync_event_handlers.rs" -o "${BUILD_DIR}/03_unsound_sync_event_handlers"
"${BUILD_DIR}/03_unsound_sync_event_handlers"

# 4. get_ip_info NULL out-pointer
echo ""
echo "--- [Fixture 04] get_info() NULL out-pointer bug ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/04_get_ip_info_null_outptr.rs" \
    -L native="${BUILD_DIR}" -l static=mock_openconnect \
    -o "${BUILD_DIR}/04_get_ip_info_null_outptr"
"${BUILD_DIR}/04_get_ip_info_null_outptr"

# 5. NULL CStr::from_ptr SIGSEGV
echo ""
echo "--- [Fixture 05] NULL CStr::from_ptr SIGSEGV (4 functions) ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/05_null_cstr_ub.rs" \
    -L native="${BUILD_DIR}" -l static=mock_openconnect \
    -o "${BUILD_DIR}/05_null_cstr_ub"
"${BUILD_DIR}/05_null_cstr_ub"

# 6. form.rs: infinite loop, slice OOB panic, and heap double-free (verified with Valgrind)
echo ""
echo "--- [Fixture 06] form.rs: double-free, infinite loop, slice OOB panic ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/06_form_bugs.rs" \
    -L native="${BUILD_DIR}" -l static=mock_openconnect \
    -o "${BUILD_DIR}/06_form_bugs"
"${BUILD_DIR}/06_form_bugs"

set +e
DF_OUTPUT=$(valgrind --error-exitcode=99 "${BUILD_DIR}/06_form_bugs" --double-free 2>&1)
DF_RC=$?
set -e
if echo "${DF_OUTPUT}" | grep -q "Invalid free()"; then
    echo "[06a] PROVEN (Valgrind heap double-free in match_choice_label, exit=${DF_RC}):"
    echo "${DF_OUTPUT}" | grep -E "Invalid free\(\)|Address .* is 0 bytes inside|mock_free_select_opt|ERROR SUMMARY" | head -n 8
else
    echo "[06a] UNEXPECTED: rc=${DF_RC}"
    echo "${DF_OUTPUT}"
    exit 1
fi

# 7. protocols.rs C heap memory leak (counter + Valgrind)
echo ""
echo "--- [Fixture 07] protocols.rs C heap memory leak ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/07_protocols_memory_leak.rs" \
    -L native="${BUILD_DIR}" -l static=mock_openconnect \
    -o "${BUILD_DIR}/07_protocols_memory_leak"
"${BUILD_DIR}/07_protocols_memory_leak"
VG_LEAK=$(valgrind --leak-check=full "${BUILD_DIR}/07_protocols_memory_leak" 2>&1 | grep -E "definitely lost|mock_openconnect_get_supported_protocols" || true)
echo "[07] Valgrind confirmation:"
echo "${VG_LEAK}"

# 8. IpInfo::from(&oc_ip_info) safe API soundness hole
echo ""
echo "--- [Fixture 08] Safe API soundness hole: IpInfo::from(&oc_ip_info) ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/08_ip_info_safe_from_soundness.rs" -o "${BUILD_DIR}/08_ip_info_safe_from_soundness"
"${BUILD_DIR}/08_ip_info_safe_from_soundness"

# 9. daemon.rs FD 0/1/2 reuse corruption
echo ""
echo "--- [Fixture 09] daemon.rs FD 0/1/2 close without /dev/null ---"
rustc --edition=2021 -g "${SCRIPT_DIR}/09_daemon_fd_reuse.rs" -o "${BUILD_DIR}/09_daemon_fd_reuse"
"${BUILD_DIR}/09_daemon_fd_reuse"

echo ""
echo "============================================================"
echo " ALL 9 SAFETY FIXTURES PROVEN SUCCESSFULLY!"
echo "============================================================"
