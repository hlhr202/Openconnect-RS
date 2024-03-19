# OIDC config for ocserv

The following is an example of how to configure ocserv to use OpenID Connect

Pre-built binary of ocserv does not include OpenID Connect support. You will need to build ocserv by yourself.

The README is based on official documentation from the ocserv project. The official documentation can be found at [https://gitlab.com/openconnect/ocserv](https://gitlab.com/openconnect/ocserv)

## Prerequisites

- Debian / Ubuntu

  ```bash
  # Required
  apt-get install -y libgnutls28-dev libev-dev
  # Optional functionality and testing
  apt-get install -y libpam0g-dev liblz4-dev libseccomp-dev \
      libreadline-dev libnl-route-3-dev libkrb5-dev libradcli-dev \
      libcurl4-gnutls-dev libcjose-dev libjansson-dev liboath-dev \
      libprotobuf-c-dev libtalloc-dev libhttp-parser-dev protobuf-c-compiler \
      gperf iperf3 lcov libuid-wrapper libpam-wrapper libnss-wrapper \
      libsocket-wrapper gss-ntlmssp haproxy iputils-ping freeradius \
      gawk gnutls-bin iproute2 yajl-tools tcpdump
  ```

- Fedora / RHEL

  ```bash
  # Required
  yum install -y gnutls-devel libev-devel
  # Optional functionality and testing
  yum install -y pam-devel lz4-devel libseccomp-devel readline-devel \
      libnl3-devel krb5-devel radcli-devel libcurl-devel cjose-devel \
      jansson-devel liboath-devel protobuf-c-devel libtalloc-devel \
      http-parser-devel protobuf-c gperf iperf3 lcov uid_wrapper \
      pam_wrapper nss_wrapper socket_wrapper gssntlmssp haproxy iputils \
      freeradius gawk gnutls-utils iproute yajl tcpdump
  ```

## Build ocserv with OpenID Connect support

In order to use OIDC with ocserv, you will need to build ocserv with the `--enable-oidc` option. This option is not enabled by default.

- clone the ocserv repository `git clone https://gitlab.com/openconnect/ocserv`

- generate the configure script `./autogen.sh`

- generate ocserv Makefile with OIDC support `./configure --enable-oidc`

- build ocserv `make`

## Prepare the OIDC configuration

The following doc is based on the official OIDC config doc from the ocserv project. The official documentation can be found at [https://gitlab.com/openconnect/ocserv/-/blob/master/doc/README-oidc.md](https://gitlab.com/openconnect/ocserv/-/blob/master/doc/README-oidc.md)

- Prepare OIDC configuration under `/etc/ocserv/conf/oidc.json`

  ```json
  {
    "openid_configuration_url": "<uri of openid-configuration doc>",
    "user_name_claim": "preferred_username",
    "required_claims": {
      "aud": "SomeAudience, should be the client_id from the OIDC provider",
      "iss": "SomeIssuer, should be the issuer URL from the OIDC provider"
    }
  }
  ```

- Edit `/etc/ocserv/ocserv.conf` to include the OIDC configuration.

  Do not forget to comment out any other `auth` configuration since only one `auth` method can be used at a time.

  ```conf
  auth = "oidc[config=/etc/ocserv/conf/oidc.json]"
  ```

- Start ocserv using the binary built in the previous step

  ```bash
  ocserv -c /etc/ocserv/ocserv.conf
  ```
