#!/usr/bin/env bash
# FindIPSec FortiGate VPN Client - Debian / Ubuntu / Pop!_OS / Linux Mint Installer

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../../" && pwd)"

if [[ "${EUID}" -eq 0 ]]; then
    error "Do not run this script as root/sudo directly. It will request sudo when required."
fi

info "Starting Debian/Ubuntu family installation for FindIPSec VPN..."

# 1. Install prerequisites
info "Updating apt index and installing build dependencies..."
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev \
    polkitd pkexec cargo nodejs npm strongswan charon-cmd libsecret-1-dev

# 2. Build binaries using Cargo and NPM
cd "${WORKSPACE_ROOT}"
info "Installing NPM dependencies..."
npm install

info "Compiling FindIPSec VPN binaries (release)..."
npx tauri build --no-bundle

# 3. System Installation
info "Installing binaries and Polkit policies..."
sudo install -Dm755 "target/release/findipsec-vpn-gui" "/usr/bin/findipsec-vpn-gui"
sudo install -Dm755 "target/release/findipsec-vpn-helper" "/usr/bin/findipsec-vpn-helper"
sudo install -Dm644 "packaging/polkit/pt.findipsec.vpn.helper.policy" "/usr/share/polkit-1/actions/pt.findipsec.vpn.helper.policy"
sudo install -Dm644 "packaging/arch/findipsec-vpn.desktop" "/usr/share/applications/findipsec-vpn.desktop"
sudo install -Dm644 "packaging/arch/findipsec-vpn.svg" "/usr/share/icons/hicolor/scalable/apps/findipsec-vpn.svg"

success "FindIPSec VPN installed successfully!"

echo ""
echo "========================================================="
echo "  FindIPSec VPN Installation Complete!"
echo "========================================================="
echo "Launch FindIPSec VPN from your desktop applications menu, or run:"
echo "    findipsec-vpn-gui"
echo ""
