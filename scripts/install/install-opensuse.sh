#!/usr/bin/env bash
# Findmore FortiGate VPN Client - openSUSE Tumbleweed / Leap Installer

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }

# 1. Verify Zypper package manager
if ! command -v zypper >/dev/null 2>&1; then
    error "This script requires an openSUSE system with zypper."
fi

# 2. Find repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../" && pwd)"
cd "${REPO_ROOT}"

info "Starting openSUSE family installation for Findmore VPN..."

# 3. Install required build and system dependencies
info "Installing required packages via zypper..."
sudo zypper --non-interactive install \
    gcc \
    gcc-c++ \
    make \
    pkg-config \
    libopenssl-devel \
    webkit2gtk-4_1-devel \
    libsecret-devel \
    polkit \
    strongswan \
    curl \
    git || sudo zypper --non-interactive install webkit2gtk3-devel

# Check node & npm
if ! command -v npm >/dev/null 2>&1; then
    info "Installing Node.js & NPM..."
    sudo zypper --non-interactive install nodejs npm
fi

# Check rust & cargo
if ! command -v cargo >/dev/null 2>&1; then
    info "Cargo not found in PATH. Installing cargo..."
    sudo zypper --non-interactive install cargo || {
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

success "openSUSE installation complete!"
echo
echo "Launch Findmore VPN from your desktop applications menu, or run:"
echo "    findmore-vpn-gui"
echo
