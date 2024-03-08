# Build guide

## Pre-request

according to the openconnect [build guide](https://www.infradead.org/openconnect/building.html), you should install the following packages as dependencies.

### Download vpnc-script if you don't have it

You can test if vpnc-script exists in your system by `ls /etc/vpnc/vpnc-script` or `ls /usr/share/vpnc-scripts/vpnc-script`. If it doesn't exist, you can download it from the openconnect project.

```bash
# you may need to run under sudo
mkdir -p /opt/vpnc-scripts
curl -o /opt/vpnc-scripts/vpnc-script https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script
chmod +x /opt/vpnc-scripts/vpnc-script
```

### For Ubuntu

```bash
apt install libxml2
apt install zlib1g zlib1g-dev
apt install openssl libssl-dev
apt install pkg-config
```

For building tools:

```bash
apt install automake # for aclocal
apt install libtool # for libtoolize
apt install gettext # for msgfmt
```

### For MacOS

```bash
brew install libxml2
brew install zlib
brew install openssl
brew install pkg-config
```

For building tools:

```bash
brew install automake # for aclocal
brew install libtool # for libtool
brew install gettext # for msgfmt
```

## Download source

```bash
git clone https://gitlab.com/openconnect/openconnect.git
```

## Build static library

```bash
cd openconnect
./autogen.sh
./configure --enable-shared --enable-static \
    --with-openssl \
    --without-gssapi \
    --without-libproxy \
    --without-stoken \
    --without-libpcsclite \
    --without-libpskc \
    --without-gnutls
    # --with-vpnc-script=/opt/vpnc-scripts/vpnc-script
# probably you need to add --with-vpnc-script=/opt/vpnc-scripts/vpnc-script
make
```
