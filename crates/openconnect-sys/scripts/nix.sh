#!/usr/bin/env sh
set -eu

DIR="$1"
VPNC_SCRIPT="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script"
WITH_VPNC_SCRIPT="./vpnc-script"
CONFIGURE_SHARED_FLAG="--enable-shared"

IS_MSYS=0
case "${MSYSTEM:-}${OSTYPE:-}" in
    MINGW*|MSYS*|msys*)
        IS_MSYS=1
        if command -v cygpath >/dev/null 2>&1; then
            DIR="$(cygpath -u "$DIR")"
        fi
        VPNC_SCRIPT="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script-win.js"
        WITH_VPNC_SCRIPT="./vpnc-script-win.js"
        CONFIGURE_SHARED_FLAG="--disable-shared"
        export PATH="/mingw64/bin:/usr/local/bin:/usr/bin:/bin:${PATH}"
        export ACLOCAL_PATH="/mingw64/share/aclocal:/usr/share/aclocal"
        export PKG_CONFIG_PATH="/mingw64/lib/pkgconfig:/mingw64/share/pkgconfig"
        export PKG_CONFIG_SYSTEM_INCLUDE_PATH="/mingw64/include"
        export PKG_CONFIG_SYSTEM_LIBRARY_PATH="/mingw64/lib"
        export CFLAGS="${CFLAGS:--O2} -std=gnu17 -Wno-incompatible-pointer-types -Wno-int-conversion -Wno-deprecated-declarations"
        ;;
esac

if [ -d "$DIR" ] && [ -f "$DIR/.libs/libopenconnect.a" ]; then
    echo "build openconnect-sys: openconnect static library already exists"
    exit 0
fi

rm -rf "$DIR"

if [ "$(uname -s 2>/dev/null || true)" = "Darwin" ]; then
    export PATH="/opt/homebrew/opt/gettext/bin:/usr/local/opt/gettext/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"
    export ACLOCAL_PATH="/opt/homebrew/opt/gettext/share/aclocal:/usr/local/opt/gettext/share/aclocal:/opt/homebrew/share/aclocal:/usr/local/share/aclocal${ACLOCAL_PATH:+:$ACLOCAL_PATH}"
    export PKG_CONFIG_PATH="/opt/homebrew/opt/openssl@3/lib/pkgconfig:/opt/homebrew/opt/openssl/lib/pkgconfig:/opt/homebrew/opt/libxml2/lib/pkgconfig:/opt/homebrew/opt/zlib/lib/pkgconfig:/usr/local/opt/openssl@3/lib/pkgconfig:/usr/local/opt/openssl/lib/pkgconfig:/usr/local/opt/libxml2/lib/pkgconfig:/usr/local/opt/zlib/lib/pkgconfig:/opt/homebrew/lib/pkgconfig:/usr/local/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi

echo "build openconnect-sys: cloning openconnect v9.12"
git clone --depth 1 --branch v9.12 https://gitlab.com/openconnect/openconnect.git "$DIR"

cd "$DIR"

echo "build openconnect-sys: downloading vpnc-script"
curl -fsSLO "$VPNC_SCRIPT"

echo "build openconnect-sys: generating configure"
./autogen.sh

echo "build openconnect-sys: configuring"
./configure "$CONFIGURE_SHARED_FLAG" --enable-static \
    --disable-nls \
    --with-openssl \
    --without-gssapi \
    --without-libproxy \
    --without-stoken \
    --without-libpcsclite \
    --without-libpskc \
    --without-gnutls \
    --with-vpnc-script="$WITH_VPNC_SCRIPT"

echo "build openconnect-sys: making"
make clean
JOBS="$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 2)"
if [ "$IS_MSYS" -eq 1 ]; then
    echo "#define LIBXML_STATIC" >> config.h
    make -j"$JOBS" libopenconnect.la
else
    make -j"$JOBS"
fi
