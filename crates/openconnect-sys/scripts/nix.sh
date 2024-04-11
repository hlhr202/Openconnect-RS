DIR=$1
VPNC_SCRIPT="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script"
WITH_VPNC_SCRIPT="./vpnc-script"

if [ -d $DIR ]; then
    echo "build openconnect-sys: openconnect folder already exists"
else
    if [[ "$OSTYPE" == "msys" ]]; then
        VPNC_SCRIPT="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script-win.js"
        WITH_VPNC_SCRIPT="./vpnc-script-win.js"
    fi

    echo "build openconnect-sys: cloning openconnect"
    git clone https://gitlab.com/openconnect/openconnect.git $DIR

    cd $DIR

    echo "build openconnect-sys: downloading vpnc-script"
    curl -O $VPNC_SCRIPT

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
        --with-vpnc-script=$WITH_VPNC_SCRIPT

    echo "build openconnect-sys: making"
    make clean
    if [[ "$OSTYPE" == "msys" ]]; then
        echo "#define LIBXML_STATIC" >>config.h
    fi
    make

fi
