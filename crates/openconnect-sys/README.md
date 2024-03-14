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

### For Windows (MSYS2 MINGW64)

Download MSYS2

Switch rust toolchain to windows-gnu

```bash
rustup default stable-x86_64-pc-windows-gnu
```

You have to use MSYS2 MINGW64 shell to build the library for 64-bit Windows.

```bash
pacman -S base-devel
pacman -S automake
pacman -S libtool
pacman -S gettext
pacman -S autotools
pacman -S pkg-config

pacman -S libxml2 libxml2-devel
pacman -S zlib
pacman -S openssl openssl-devel
pacman -S icu icu-devel
```

### Minor changes for Windows

- in config.h, you may add

  ```c
  // TODO: check if it's necessary when building under MSYS2 MINGW64
  #define LIBXML_STATIC
  ```

- for cargo build, you have to use CFLAGS under MSYS2 MINGW64 shell, this avoid error when building rustls.
  See details here: https://github.com/aws/aws-lc-rs/issues/370

  For bash:

  ```bash
  export CFLAGS="-D_ISOC11_SOURCE" && cargo build
  ```

  For powershell:

  ```powershell
  $env:CFLAGS="-D_ISOC11_SOURCE"; cargo build
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
    # --with-vpnc-script=./vpnc-script-win.js # for windows
    # --with-vpnc-script=/opt/vpnc-scripts/vpnc-script # for *nix without vpnc-script installed
make
```
