#!/bin/sh

download() {

    mkdir -p ./artifacts

    # download the artifact by the following commands, select the all openconnect-xx artifacts
    gh run download --dir ./artifacts

    mv ./artifacts/openconnect-linux-x64/openconnect-cli ./artifacts/openconnect-linux-x64/openconnect-cli_linux-x86_64
    mv ./artifacts/openconnect-mac-aarch64/openconnect-cli ./artifacts/openconnect-mac-aarch64/openconnect-cli_osx-aarch64
    mv ./artifacts/openconnect-mac-x64/openconnect-cli ./artifacts/openconnect-mac-x64/openconnect-cli_osx-x86_64
}

clean() {
    rm -rf ./artifacts
}

if [ "$1" = "download" ]; then
    download
elif [ "$1" = "clean" ]; then
    clean
else
    echo "Usage: $0 {download|clean}"
    exit 1
fi
