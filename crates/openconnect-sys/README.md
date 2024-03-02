# Build guide

## Pre-request (Ubuntu)

according to the openconnect [build guide](https://www.infradead.org/openconnect/building.html), you should install the following packages as dependencies.

```bash
apt install libxml2
apt install zlib1g zlib1g-dev
apt install openssl libssl-dev
apt install pkg-config
```

also, you should install the following packages for building openconnect.

```bash
apt install automake # for aclocal
apt install libtool # for libtoolize
apt install gettext # for msgfmt
```

## Download source

```bash
git clone https://gitlab.com/openconnect/openconnect.git
```

## Build static library

```bash
cd openconnect
./autogen.sh
./configure --enable-static --disable-shared
make
```