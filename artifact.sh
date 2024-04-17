#!/bin/sh

CURRENT_DIR=$(pwd)

download() {

    mkdir -p ./artifacts

    # # download the artifact by the following commands, select the all openconnect-xx artifacts
    gh run download --dir ./artifacts

    mv ./artifacts/openconnect-linux-x64/openconnect-cli ./artifacts/openconnect-linux-x64/openconnect-cli_linux-x86_64
    mv ./artifacts/openconnect-mac-aarch64/openconnect-cli ./artifacts/openconnect-mac-aarch64/openconnect-cli_osx-aarch64
    mv ./artifacts/openconnect-mac-x64/openconnect-cli ./artifacts/openconnect-mac-x64/openconnect-cli_osx-x86_64

    CODESIGN_IDENTITY=$(security find-identity -p codesigning | grep "CSSMERR_TP_NOT_TRUSTED" | awk '{print $3}' | tr -d '"')

    # process the macos aaarch64 bundle
    cd ./artifacts/openconnect-mac-aarch64/bundle/macos
    chmod +x ./openconnect-gui.app/Contents/MacOS/openconnect-gui
    codesign -fs "$CODESIGN_IDENTITY" ./openconnect-gui.app
    create-dmg \
        --volname "Openconnect GUI" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "openconnect-gui.app" 200 190 \
        --hide-extension "openconnect-gui.app" \
        --app-drop-link 600 185 \
        "openconnect-gui_osx-aarch64.dmg" \
        "openconnect-gui.app/"

    cd $CURRENT_DIR

    # process the macos x86_64 bundle
    cd ./artifacts/openconnect-mac-x64/bundle/macos
    chmod +x ./openconnect-gui.app/Contents/MacOS/openconnect-gui
    codesign -fs "$CODESIGN_IDENTITY" ./openconnect-gui.app
    create-dmg \
        --volname "Openconnect GUI" \
        --window-pos 200 120 \
        --window-size 800 400 \
        --icon-size 100 \
        --icon "openconnect-gui.app" 200 190 \
        --hide-extension "openconnect-gui.app" \
        --app-drop-link 600 185 \
        "openconnect-gui_osx-x86_64.dmg" \
        "openconnect-gui.app/"

    cd $CURRENT_DIR
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
