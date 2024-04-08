DIR=$1

if [ -d $DIR ]; then
    echo "build openconnect-sys: openconnect folder already exists"
else
    echo "build openconnect-sys: cloning openconnect"

    git clone https://gitlab.com/openconnect/openconnect.git $DIR
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
    make
fi
