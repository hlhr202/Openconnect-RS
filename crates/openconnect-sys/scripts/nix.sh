DIR=$1
LIB_PATH="$DIR/.libs/libopenconnect.a"

if [ -e $LIB_PATH ]; then
    echo "build openconnect-sys: openconnect folder already exists"
else
    cd $DIR

    echo "build openconnect-sys: downloading vpnc-script"
    curl -O https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script

    echo "build openconnect-sys: generating configure"
    ./autogen.sh

    echo "build openconnect-sys: configuring"
    ./configure --enable-shared --enable-static \
        --with-openssl \
        --without-gssapi \
        --without-libproxy \
        --without-stoken \
        --without-libpcsclite \
        --without-libpskc \
        --without-gnutls \
        --with-vpnc-script=./vpnc-script

    echo "build openconnect-sys: making"
    make clean
    make

    git clean -fd
fi
