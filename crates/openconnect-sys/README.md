# Build guide

## Pre-request

according to the openconnect [build guide](https://www.infradead.org/openconnect/building.html), you should install the following packages as dependencies.

### For Ubuntu

```bash
apt install libxml2
apt install zlib1g zlib1g-dev
apt install openssl libssl-dev
apt install liblz4-dev liblzma-dev
apt install pkg-config
```

For building tools (if you don't want to use the prebuilt openconnect):

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

For building tools (if you don't want to use the prebuilt openconnect):

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
pacman -S base-devel mingw-w64-x86_64-toolchain
pacman -S automake
pacman -S libtool
pacman -S gettext
pacman -S autotools
pacman -S pkg-config

pacman -S openssl openssl-devel mingw-w64-x86_64-openssl
pacman -S libxml2 libxml2-devel mingw-w64-x86_64-libxml2
pacman -S libiconv libiconv-devel mingw-w64-x86_64-libiconv
pacman -S zlib zlib-devel mingw-w64-x86_64-zlib
pacman -S liblz4 liblz4-devel mingw-w64-x86_64-lz4
pacman -S liblzma liblzma-devel mingw-w64-x86_64-xz
pacman -S icu icu-devel mingw-w64-x86_64-icu
```

### Related to prebuilt openconnect

By default, we use the prebuilt openconnect static lib, which is downloaded from [sourceforge](https://sourceforge.net/projects/openconnect-prebuilt/files/).

If you want to build the openconnect static lib when building the crate, you can add environment variable to your .cargo/config file.

```toml
[env]
OPENCONNECT_USE_PREBUILT = "false"
```

A further investigation of manual build can be found in [MANUAL_BUILD.md](MANUAL_BUILD.md).