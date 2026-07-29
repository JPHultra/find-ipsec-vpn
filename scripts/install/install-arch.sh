#!/usr/bin/env bash
# FindIPSec FortiGate VPN Client - Arch Linux / Omarchy / Manjaro Installer

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

info "Starting Arch Linux / Omarchy installation for FindIPSec VPN..."

# 1. Install build dependencies
info "Installing build and runtime dependencies via pacman..."
sudo pacman -S --needed --noconfirm base-devel cargo npm nodejs gtk3 webkit2gtk-4.1 polkit openssl

# 2. Build strongswan-fortigate package
info "Building custom strongSwan 6.0.7 with FortiGate XAuth passcode patch..."
ARCH_PKG_DIR="${WORKSPACE_ROOT}/packaging/arch"
cd "${ARCH_PKG_DIR}"

if ! makepkg -f -p strongswan-fortigate.PKGBUILD -si --noconfirm; then
    error "Failed to build or install strongswan-fortigate package."
fi

success "strongswan-fortigate installed successfully!"

# 3. Build and Install findipsec-vpn GUI
info "Building and installing FindIPSec VPN GUI client..."
cd "${ARCH_PKG_DIR}"

if ! makepkg -f -p findipsec-vpn.PKGBUILD -si --noconfirm; then
    error "Failed to build or install findipsec-vpn."
fi

success "FindIPSec VPN GUI client installed successfully!"

echo ""
echo "========================================================="
echo "  FindIPSec VPN Installation Complete!"
echo "========================================================="
echo "Launch FindIPSec VPN from your applications menu or run:"
echo "    findipsec-vpn-gui"
echo ""
