#!/usr/bin/env bash
# Findmore FortiGate VPN Client - Debian / Ubuntu / Pop!_OS / Linux Mint Installer

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }

# 1. Verify APT package manager
if ! command -v apt-get >/dev/null 2>&1; then
    error "This script requires a Debian/Ubuntu based system with apt-get."
fi

# 2. Find repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../" && pwd)"
cd "${REPO_ROOT}"

info "Starting Debian/Ubuntu family installation for Findmore VPN..."

# 3. Install required build and system dependencies
info "Installing required packages via apt..."
sudo apt-get update -qq
sudo apt-get install -y -qq \
    build-essential \
    pkg-config \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libsecret-1-dev \
    policykit-1 \
    strongswan \
    charon-cmd \
    curl \
    git

# Check node & npm
if ! command -v npm >/dev/null 2>&1; then
    info "Installing Node.js & NPM..."
    sudo apt-get install -y -qq nodejs npm
fi

# Check rust & cargo
if ! command -v cargo >/dev/null 2>&1; then
    info "Cargo not found in PATH. Installing rustup/cargo..."
    sudo apt-get install -y -qq cargo || {
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    }
fi

# 4. Install NPM frontend dependencies
info "Installing Node dependencies..."
npm install

# 5. Build Tauri application in release mode
info "Compiling Findmore VPN binaries (release)..."
npx tauri build --no-bundle

# 6. Deploy binaries, Polkit policy, desktop entry, and icons
info "Installing binaries and desktop integration files..."
sudo install -Dm755 "target/release/findmore-vpn-gui" "/usr/bin/findmore-vpn-gui"
sudo install -Dm755 "target/release/findmore-vpn-helper" "/usr/bin/findmore-vpn-helper"
sudo install -Dm644 "packaging/polkit/pt.findmore.vpn.helper.policy" "/usr/share/polkit-1/actions/pt.findmore.vpn.helper.policy"
sudo install -Dm644 "packaging/arch/findmore-vpn.desktop" "/usr/share/applications/findmore-vpn.desktop"
sudo install -Dm644 "packaging/arch/findmore-vpn.svg" "/usr/share/icons/hicolor/scalable/apps/findmore-vpn.svg"

# Update desktop database
if command -v update-desktop-database >/dev/null 2>&1; then
    sudo update-desktop-database /usr/share/applications/ 2>/dev/null || true
fi

success "Debian / Ubuntu installation complete!"
echo
echo "Launch Findmore VPN from your desktop applications menu, or run:"
echo "    findmore-vpn-gui"
echo
