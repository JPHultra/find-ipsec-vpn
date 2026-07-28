#!/usr/bin/env bash

set -euo pipefail

# Text colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    echo -e "${BLUE}==>${NC} $*"
}

success() {
    echo -e "${GREEN}==>${NC} $*"
}

error() {
    echo -e "${RED}ERROR:${NC} $*" >&2
    exit 1
}

# 1. Verify Arch Linux / Pacman / makepkg
if ! command -v pacman >/dev/null 2>&1 || ! command -v makepkg >/dev/null 2>&1; then
    error "This installation script is designed for Arch Linux and derived systems (e.g. Omarchy) only."
fi

# 2. Verify repository root
if [[ ! -d "packaging/arch" || ! -f "package.json" ]]; then
    error "Please run this script from the root of the findmore-vpn repository directory."
fi

# 3. Import strongSwan release GPG key if not present
info "Checking strongSwan release signing key..."
if ! gpg --list-keys DF42C170B34DBA77 >/dev/null 2>&1; then
    info "Importing strongSwan signing key (DF42C170B34DBA77)..."
    gpg --keyserver hkps://keys.openpgp.org --recv-keys DF42C170B34DBA77 || \
    gpg --keyserver hkps://keyserver.ubuntu.com --recv-keys DF42C170B34DBA77 || \
    error "Could not import strongSwan signing key. Please import it manually: gpg --recv-keys DF42C170B34DBA77"
else
    success "strongSwan signing key is already present."
fi

# 4. Build and Install strongswan-fortigate
info "Building and installing custom patched strongSwan package (strongswan-fortigate)..."
cd packaging/arch

# Clean previous build directories to ensure no permission blocks
rm -rf src/ pkg/

if ! makepkg -f -p strongswan-fortigate.PKGBUILD -si; then
    error "Failed to build or install strongswan-fortigate."
fi
success "strongswan-fortigate installed successfully."

# 5. Build and Install findmore-vpn GUI
info "Building and installing Findmore VPN GUI client..."
# Clean previous build directories to ensure no permission blocks
rm -rf src/ pkg/

if ! makepkg -f -p findmore-vpn.PKGBUILD -si; then
    error "Failed to build or install findmore-vpn."
fi
success "Findmore VPN GUI client installed successfully!"

# 6. Reset permissions to avoid IDE indexer locks
chmod -R u+rwX pkg/ src/ 2>/dev/null || true

cd ../../

success "Installation complete!"
echo
echo "You can launch the VPN Client from your desktop applications menu, or by running:"
echo "    findmore-vpn-gui"
echo
