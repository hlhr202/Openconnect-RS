#!/bin/bash
# Installation script for Openconnect-RS CLI
# This script will download the Openconnect-RS CLI and vpnc-script and install them in $HOME/.oidcvpn/bin
# It will also add $HOME/.oidcvpn/bin to PATH
# Usage:
# curl -s -L URL_TO_SCRIPT_HERE | sh

COLOR_PRIMARY="\033[0;34m"
COLOR_WARN="\033[1;33m"
COLOR_SUCCESS="\033[0;32m"
COLOR_RESET="\033[0m"

echo ""
echo "=================================="
echo ""
echo "${COLOR_PRIMARY}Installing Openconnect-RS CLI${COLOR_RESET}"
echo ""
echo ""
echo "This script will download the Openconnect CLI and vpnc-script and install them in $HOME/.oidcvpn/bin"
echo "${COLOR_WARN}WARNING: Openconnect-RS CLI has the same installed binary name as the original Openconnect CLI."
echo "Please remove the original Openconnect CLI if you wish to use Openconnect-RS CLI.${COLOR_RESET}"
echo ""
echo "=================================="
echo ""

# shut down if openconnect is running
if pgrep -x "openconnect" > /dev/null
then
    echo "Openconnect is running. Please shut it down before installing Openconnect-RS CLI"
    exit 1
fi

CLI_DOWNLOAD_URL=""
VPNC_SCRIPT_URL="https://gitlab.com/openconnect/vpnc-scripts/raw/master/vpnc-script"

# detect os
if [[ "$OSTYPE" = "darwin"* ]]; then
    # detect arch
    if [[ "$HOSTTYPE" = "x86_64" ]]; then
        # install macos cli
        CLI_DOWNLOAD_URL="https://github.com/hlhr202/Openconnect-RS/releases/download/v0.0.0-pre1/openconnect-cli_osx-x86_64"
    elif [[ "$HOSTTYPE" = "arm64" ]]; then
        # install macos cli
        CLI_DOWNLOAD_URL="https://github.com/hlhr202/Openconnect-RS/releases/download/v0.0.0-pre1/openconnect-cli_osx-aarch64"
    else
        echo "unsupported arch"
        exit 1
    fi

elif [[ "$OSTYPE" = "linux-gnu" ]]; then
    if [[ "$HOSTTYPE" = "x86_64" ]]; then
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
if [[ ! -d "$HOME/.oidcvpn/bin" ]]; then
    mkdir -p $HOME/.oidcvpn/bin
fi

# download cli
echo "${COLOR_PRIMARY}Downloading cli${COLOR_RESET}"
echo ""
curl -L $CLI_DOWNLOAD_URL >$HOME/.oidcvpn/bin/openconnect
chmod +x $HOME/.oidcvpn/bin/openconnect
echo ""
echo "=================================="
echo ""

# download vpnc-script
echo "${COLOR_PRIMARY}Downloading vpnc-script${COLOR_RESET}"
echo ""
curl -L $VPNC_SCRIPT_URL >$HOME/.oidcvpn/bin/vpnc-script
chmod +x $HOME/.oidcvpn/bin/vpnc-script
echo ""
echo "=================================="
echo ""

# add .oidcvpn/bin to PATH
echo "${COLOR_PRIMARY}Adding .oidcvpn/bin to PATH${COLOR_RESET}"
echo ""
if [[ ":$PATH:" != *":$HOME/.oidcvpn/bin:"* ]]; then

    echo "Adding .oidcvpn/bin to PATH"

    # check if .bashrc or .bash_profile exists
    if [[ -f "$HOME/.bashrc" ]]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.bashrc
        echo "Run source $HOME/.bashrc to apply changes"
    elif [[ -f "$HOME/.bash_profile" ]]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.bash_profile
        echo "Run source $HOME/.bash_profile to apply changes"
    fi

    # check if .zshrc exists
    if [[ -f "$HOME/.zshrc" ]]; then
        echo "export PATH=\$PATH:$HOME/.oidcvpn/bin" >>$HOME/.zshrc
        echo "Run source $HOME/.zshrc to apply changes"
    fi

fi
echo "If you are using shell other than bash or zsh, please add the following line to your shell profile:"
echo "export PATH=\$PATH:$HOME/.oidcvpn/bin"

echo ""
echo "=================================="
echo ""

echo "${COLOR_SUCCESS}Installation complete!${COLOR_RESET}"
