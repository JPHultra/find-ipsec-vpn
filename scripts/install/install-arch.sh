#!/usr/bin/env bash
# Findmore FortiGate VPN Client - Arch Linux / Omarchy / Manjaro Installer

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }

# 1. Verify Arch Linux / pacman / makepkg
if ! command -v pacman >/dev/null 2>&1 || ! command -v makepkg >/dev/null 2>&1; then
    error "This script requires an Arch Linux based system with pacman and makepkg."
fi

# 2. Find repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../" && pwd)"
cd "${REPO_ROOT}"

info "Starting Arch Linux / Omarchy installation for Findmore VPN..."

# 3. Import strongSwan release GPG key if not present
info "Checking strongSwan release signing key..."
if ! gpg --list-keys DF42C170B34DBA77 >/dev/null 2>&1; then
    info "Importing strongSwan signing key (DF42C170B34DBA77)..."
    gpg --keyserver hkps://keys.openpgp.org --recv-keys DF42C170B34DBA77 || \
    gpg --keyserver hkps://keyserver.ubuntu.com --recv-keys DF42C170B34DBA77 || \
    error "Could not import strongSwan signing key. Please import it manually: gpg --recv-keys DF42C170B34DBA77"
else
    success "strongSwan signing key is present."
fi

# 4. Build and Install strongswan-fortigate
info "Building and installing custom patched strongSwan package (strongswan-fortigate)..."
cd packaging/arch
rm -rf src/ pkg/

if ! makepkg -f -p strongswan-fortigate.PKGBUILD -si --noconfirm; then
    error "Failed to build or install strongswan-fortigate."
fi
success "strongswan-fortigate installed successfully."

# 5. Build and Install findmore-vpn GUI
info "Building and installing Findmore VPN GUI client..."
rm -rf src/ pkg/

if ! makepkg -f -p findmore-vpn.PKGBUILD -si --noconfirm; then
    error "Failed to build or install findmore-vpn."
fi
success "Findmore VPN GUI client installed successfully!"

chmod -R u+rwX pkg/ src/ 2>/dev/null || true
cd "${REPO_ROOT}"

success "Arch Linux / Omarchy installation complete!"
echo
echo "Launch Findmore VPN from your applications menu or run:"
echo "    findmore-vpn-gui"
echo
