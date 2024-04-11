# Manually build openconnect for static linking

## Download source

```bash
git clone https://gitlab.com/openconnect/openconnect.git
```

## Download vpnc-script if you don't have it

You can test if vpnc-script exists in your system by `ls /etc/vpnc/vpnc-script` or `ls /usr/share/vpnc-scripts/vpnc-script`. If it doesn't exist, you can download it from the openconnect project.

```bash
# you may need to run under sudo
curl -O https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script
# for windows, you should use vpnc-script-win.js
chmod +x ./vpnc-script
```

## Generate autoconf

```bash
./autogen.sh
```

## Configure openconnect

```bash
./configure --enable-shared --enable-static \
    --with-openssl \
    --without-gssapi \
    --without-libproxy \
    --without-stoken \
    --without-libpcsclite \
    --without-libpskc \
    --without-gnutls
    # --with-vpnc-script=./vpnc-script-win.js # for windows
    # --with-vpnc-script=./vpnc-script # for *nix without vpnc-script installed
```

## Minor changes for Windows

- in config.h, you have to add

  ```c
  #define LIBXML_STATIC
  ```

- for cargo build, you have to use CFLAGS under MSYS2 MINGW64 shell, this avoid error when building rustls.
  See a similar case of aws-lc-sys here: https://github.com/aws/aws-lc-rs/issues/370

  For bash:

  ```bash
  export CFLAGS="-D_ISOC11_SOURCE"
  ```

  For powershell:

  ```powershell
  $env:CFLAGS="-D_ISOC11_SOURCE";
  ```

## Build openconnect static library

```bash
make
```
