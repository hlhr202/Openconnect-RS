# Installation script for Openconnect CLI
# This script will download the Openconnect CLI and vpnc-script and install them in $HOME/.oidcvpn/bin
# It will also add $HOME/.oidcvpn/bin to PATH
# Usage:
# curl -s -L URL_TO_SCRIPT_HERE | bash

CLI_DOWNLOAD_URL=""
VPNC_SCRIPT_URL="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script"

# detect os
if [[ "$OSTYPE" == "darwin"* ]]; then
    # detect arch
    if [[ "$HOSTTYPE" == "x86_64" ]]; then
        # install macos cli
        echo "installing macos cli for x86_64"
        CLI_DOWNLOAD_URL="https://github.com/hlhr202/Openconnect-RS/releases/download/v0.0.0-pre1/openconnect-cli_osx-x86_64"
    elif [[ "$HOSTTYPE" == "arm64" ]]; then
        # install macos cli
        echo "installing macos cli for arm64"
        CLI_DOWNLOAD_URL="https://github.com/hlhr202/Openconnect-RS/releases/download/v0.0.0-pre1/openconnect-cli_osx-aarch64"
    else
        echo "unsupported arch"
        exit 1
    fi

elif [[ "$OSTYPE" == "linux-gnu" ]]; then
    if [[ "$HOSTTYPE" == "x86_64" ]]; then
        echo "installing linux cli"
        CLI_DOWNLOAD_URL="https://github.com/hlhr202/Openconnect-RS/releases/download/v0.0.0-pre1/openconnect-cli_linux-x86_64"
    else
        echo "unsupported arch"
        exit 1
    fi

else
    echo "unsupported os"
    exit 1
fi

# check if .oidcvpn/bin folder exists under home directory
if [ ! -d "$HOME/.oidcvpn/bin" ]; then
    mkdir -p $HOME/.oidcvpn/bin
fi

# download cli
echo "Downloading cli"
curl -L $CLI_DOWNLOAD_URL >$HOME/.oidcvpn/bin/openconnect
chmod +x $HOME/.oidcvpn/bin/openconnect

# download vpnc-script
echo "Downloading vpnc-script"
curl -L $VPNC_SCRIPT_URL >$HOME/.oidcvpn/bin/vpnc-script
chmod +x $HOME/.oidcvpn/bin/vpnc-script

# add .oidcvpn/bin to PATH
echo "Checking if .oidcvpn/bin is in PATH"

if [[ ":$PATH:" != *":$HOME/.oidcvpn/bin:"* ]]; then

    echo "Adding .oidcvpn/bin to PATH"

    # check if .bashrc or .bash_profile exists
    if [ -f "$HOME/.bashrc" ]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.bashrc
        echo "Run source $HOME/.bashrc to apply changes"
    elif [ -f "$HOME/.bash_profile" ]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.bash_profile
        echo "Run source $HOME/.bash_profile to apply changes"
    fi

    # check if .zshrc exists
    if [ -f "$HOME/.zshrc" ]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.zshrc
        echo "Run source $HOME/.zshrc to apply changes"
    fi

    echo "If you are using shell other than bash or zsh, please add the following line to your shell profile:"
    echo "export PATH=\$PATH:$HOME/.oidcvpn/bin"
fi

echo "Installation complete"
