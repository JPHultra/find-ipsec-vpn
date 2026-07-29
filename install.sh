#!/usr/bin/env bash
# Findmore FortiGate VPN Client - Universal Multi-Distro Installer

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${BLUE}==>${NC} $*"; }
success() { echo -e "${GREEN}==>${NC} $*"; }
warn() { echo -e "${YELLOW}WARNING:${NC} $*"; }
error() { echo -e "${RED}ERROR:${NC} $*" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

info "Detecting Linux distribution family..."

OS_ID=""
OS_LIKE=""

if [[ -f /etc/os-release ]]; then
    # Source os-release safely
    eval "$(grep -E '^(ID|ID_LIKE)=' /etc/os-release)"
    OS_ID="${ID:-}"
    OS_LIKE="${ID_LIKE:-}"
fi

info "Detected OS ID: '${OS_ID}', OS LIKE: '${OS_LIKE}'"

# Route to distro-specific installer
if [[ "${OS_ID}" =~ ^(arch|omarchy|manjaro|endeavouros|garuda)$ ]] || [[ "${OS_LIKE}" =~ arch ]]; then
    info "Launching Arch Linux / Omarchy installer..."
    exec ./scripts/install/install-arch.sh "$@"

elif [[ "${OS_ID}" =~ ^(ubuntu|debian|pop|mint|elementary|zorin)$ ]] || [[ "${OS_LIKE}" =~ (debian|ubuntu) ]]; then
    info "Launching Debian / Ubuntu installer..."
    exec ./scripts/install/install-debian.sh "$@"

elif [[ "${OS_ID}" =~ ^(fedora|rhel|rocky|almalinux|centos)$ ]] || [[ "${OS_LIKE}" =~ fedora ]]; then
    info "Launching Fedora / RHEL installer..."
    exec ./scripts/install/install-fedora.sh "$@"

elif [[ "${OS_ID}" =~ ^(opensuse|opensuse-tumbleweed|opensuse-leap|suse)$ ]] || [[ "${OS_LIKE}" =~ suse ]]; then
    info "Launching openSUSE installer..."
    exec ./scripts/install/install-opensuse.sh "$@"

else
    warn "Could not automatically match distribution family '${OS_ID}'."
    echo "Please choose your distribution family to continue:"
    echo "  1) Arch Linux / Omarchy / Manjaro"
    echo "  2) Ubuntu / Debian / Pop!_OS / Linux Mint"
    echo "  3) Fedora / RHEL / Rocky Linux"
    echo "  4) openSUSE Tumbleweed / Leap"
    read -rp "Select option [1-4]: " choice

    case "${choice}" in
        1) exec ./scripts/install/install-arch.sh "$@" ;;
        2) exec ./scripts/install/install-debian.sh "$@" ;;
        3) exec ./scripts/install/install-fedora.sh "$@" ;;
        4) exec ./scripts/install/install-opensuse.sh "$@" ;;
        *) error "Invalid choice selected." ;;
    esac
fi
