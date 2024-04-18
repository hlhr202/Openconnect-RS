#!/bin/sh

CURRENT_DIR=$(pwd)

download() {
    if [ -d "./artifacts" ]; then
        echo "The artifacts directory already exists, please remove it first."
        exit 1
    fi

    echo "Downloading the artifacts..."
    mkdir -p ./artifacts
    # # download the artifact by the following commands, select the all openconnect-xx artifacts
    gh run download --dir ./artifacts
    echo ""

    echo "Renaming the CLI binaries..."
    mv ./artifacts/openconnect-linux-x64/openconnect-cli ./artifacts/openconnect-linux-x64/openconnect-cli_linux-x86_64
    mv ./artifacts/openconnect-mac-aarch64/openconnect-cli ./artifacts/openconnect-mac-aarch64/openconnect-cli_osx-aarch64
    mv ./artifacts/openconnect-mac-x64/openconnect-cli ./artifacts/openconnect-mac-x64/openconnect-cli_osx-x86_64
    echo ""

    if [[ "$OSTYPE" = "darwin"* ]]; then
        # codesign the macos bundle
        CODESIGN_IDENTITY=$(security find-identity -p codesigning | grep "CSSMERR_TP_NOT_TRUSTED" | awk '{print $3}' | tr -d '"')

        # process the macos aarch64 bundle
        echo "Codesigning the macos aarch64 bundle..."

        cd ./artifacts/openconnect-mac-aarch64/bundle/macos
        chmod +x ./openconnect-gui.app/Contents/MacOS/openconnect-gui
        codesign -fs "$CODESIGN_IDENTITY" ./openconnect-gui.app
        echo ""

        echo "Creating the aarch 64 dmg files..."
        # create dmg
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
        echo ""

        # process the macos x86_64 bundle
        echo "Codesigning the macos x86_64 bundle..."

        cd ./artifacts/openconnect-mac-x64/bundle/macos
        chmod +x ./openconnect-gui.app/Contents/MacOS/openconnect-gui
        codesign -fs "$CODESIGN_IDENTITY" ./openconnect-gui.app
        echo ""

        echo "Creating the x86_64 dmg files..."
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
        echo ""
    fi
}

clean() {
    echo "Cleaning the artifacts..."
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
