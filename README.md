# Findmore FortiGate VPN GUI Client

A lightweight, secure, and beautiful desktop application for connecting to the Findmore FortiGate VPN on Arch Linux (Omarchy / Hyprland / GNOME / KDE) and other Linux distributions.

This application is built using **Tauri v2 (Rust + HTML/CSS/JS)**. It provides a privilege-separated frontend to drive strongSwan's `charon-cmd` engine, interactively handle FortiGate's email OTP passcode verification, and monitor live connection metrics.

> 📘 **OS Compatibility Matrix**: See [COMPATIBILITY.md](file:///home/joao/Projects/findmore-vpn/COMPATIBILITY.md) for full feature compatibility across Ubuntu, Debian, Fedora, Arch Linux/Omarchy, and openSUSE.

---

## Key Features

### 🔒 Enterprise Security & Privilege Separation
- **Privilege-Separated Architecture**: The GUI runs completely unprivileged in user space. All IPsec tunneling and route manipulation operations are executed by an elevated child helper (`findmore-vpn-helper`) spawned securely via `pkexec`.
- **POSIX PTY Credential Passing**: Allocates pseudo-terminals (`libc::posix_openpt`) so strongSwan authentication prompts receive credentials programmatically without command-line parameters or process argument leakage.
- **OS Keyring Integration**: Pre-Shared Keys (PSK) and user account passwords are saved directly into the Linux OS Keyring (`libsecret` / Gnome Keyring / KWallet) with restricted `0600` file fallback.
- **Keyring Security Badge**: Visual status indicator in the profile editor indicating when credentials are hardware/keyring secured.

### 🌐 Connection & Profile Management
- **Multi-Profile Support**: Save, edit, switch, or delete multiple VPN connection profiles for different gateways and server environments.
- **Interactive 2FA / Email OTP**: Seamlessly intercepts FortiGate `XAUTH_PASSCODE` challenges and provides a dedicated security code input interface.
- **Auto-Reconnect on Drop**: Optional "Auto-reconnect on drop" mode that automatically re-establishes lost VPN tunnels after a 3-second delay.

### 📊 Real-Time Metrics & Diagnostic Logs
- **Live Traffic Monitoring**: Queries Linux kernel IPsec Security Association tables (`ip -s xfrm state`) every 1.5 seconds to report real-time byte throughput (Sent / Received) and uptime counters.
- **Split Log Drawer**: Access real-time **Session Logs** and **Engine Logs**.
- **One-Click Copy & Export**: Copy log buffers directly to your clipboard or export combined diagnostic files (`findmore-vpn-logs-YYYY-MM-DD.log`).

### 📌 System Tray & Desktop Integration
- **System Tray Dropdown**: Displays live connection state (e.g. `Status: Connected (10.7.1.11)`) and live traffic throughput (e.g. `Traffic: 1.25 MB ↓ / 340.00 KB ↑`) directly in the tray context menu.
- **Background Minimization**: Closing the application window hides the GUI to the system tray (`libayatana-appindicator`) without interrupting active VPN tunnels.
- **Native OS Desktop Notifications**: Emits native desktop notifications for 2FA verification prompts, connection status changes, unexpected drops, and error alerts.
- **Single Instance Enforcement**: Guarantees only a single application instance runs at a time. Launching a second instance focuses the existing window.
- **Silent Startup CLI Flags**: Launch the application silently in the system tray using `--tray` or `-t` CLI flags.

---

## Architecture Design

```
┌────────────────────────────────────────┐
│        Findmore VPN GUI (User)         │
│  - Profile selector & credential store │
│  - System tray & live traffic metrics  │
│  - Secure OS Keyring integration       │
└───────────────────┬────────────────────┘
                    │
                    │ stdin/stdout (JSON Lines)
                    ▼
┌────────────────────────────────────────┐
│   findmore-vpn-helper (Root/pkexec)    │
│  - Spawns charon-cmd inside POSIX PTY  │
│  - Auto-fills PSK and password        │
│  - Intercepts and yields OTP prompt   │
│  - Parses Linux kernel XFRM stats     │
└────────────────────────────────────────┘
```

---

## Command Line Usage

```bash
# Standard launch (opens window by default)
findmore-vpn-gui

# Start silently minimized to the system tray
findmore-vpn-gui --tray
# or
findmore-vpn-gui -t
```

---

## Project Structure

```
findmore-vpn/
├── Cargo.toml                  # Workspace definition
├── COMPATIBILITY.md            # Multi-distro OS feature compatibility matrix
├── install.sh                  # Universal auto-detecting installer router
├── package.json                # Frontend package configuration
├── scripts/                    # Multi-distro installation scripts
│   └── install/
│       ├── install-arch.sh     # Arch Linux / Omarchy / Manjaro installer
│       ├── install-debian.sh   # Debian / Ubuntu / Mint / Pop!_OS installer
│       ├── install-fedora.sh   # Fedora / RHEL / Rocky installer
│       └── install-opensuse.sh # openSUSE Tumbleweed / Leap installer
├── src/                        # HTML/CSS/JS Web assets
│   ├── index.html              # Main dashboard structures
│   ├── styles.css              # Custom glassmorphic CSS styling
│   └── main.js                 # Frontend event router & state logic
├── src-tauri/                  # Desktop application backend
│   ├── Cargo.toml              # Rust backend dependencies
│   ├── tauri.conf.json         # Window size, title, and permissions
│   └── src/
│       ├── main.rs             # GUI launcher
│       ├── lib.rs              # Tauri IPC commands and security Keyring integration
│       └── helper.rs           # Privileged helper process
├── packaging/
│   ├── polkit/
│   │   └── pt.findmore.vpn.helper.policy  # Polkit XML rules for elevated execution
│   └── arch/
│       ├── findmore-vpn.desktop           # Desktop application launcher
│       ├── findmore-vpn.svg               # Application icon
│       ├── findmore-vpn.PKGBUILD          # PKGBUILD for the client application
│       ├── strongswan-fortigate.PKGBUILD   # PKGBUILD for the patched strongSwan
│       └── xauth-passcode.patch           # strongSwan patch file
└── README.md                   # Project documentation
```

---

## How to Run & Build Locally

### 1. Requirements
Ensure the following tools are installed on your Arch Linux system:
* Node.js and NPM
* Rust (`cargo` / `rustc`)
* `base-devel`, `git`

### 2. Development Mode
To launch the application in development mode with live reloading:
```bash
npm run tauri dev
```

### 3. Release Build
To compile both the GUI and the helper in release mode:
```bash
npm run tauri build -- --no-bundle
```
The output binaries are generated in `target/release/`:
- `target/release/findmore-vpn-gui`
- `target/release/findmore-vpn-helper`

---

## Multi-Distribution Linux Installation

Findmore VPN includes an automated, universal installer script that detects your Linux distribution family and deploys all dependencies, Polkit rules, binaries, and desktop icons.

For full feature matrix and desktop environment details, see [COMPATIBILITY.md](file:///home/joao/Projects/findmore-vpn/COMPATIBILITY.md).

### Quick Universal Installation (All Distros)

Run the root installation script:

```bash
./install.sh
```

The script auto-detects your operating system family and routes to the appropriate installer:

- **Arch Linux / Omarchy / Manjaro**: Runs `./scripts/install/install-arch.sh`
- **Ubuntu / Debian / Pop!_OS / Linux Mint**: Runs `./scripts/install/install-debian.sh`
- **Fedora / RHEL / Rocky Linux**: Runs `./scripts/install/install-fedora.sh`
- **openSUSE Leap / Tumbleweed**: Runs `./scripts/install/install-opensuse.sh`

### Manual Distro Script Execution

You can also execute distribution-specific installation scripts directly:

```bash
# Arch Linux / Omarchy
./scripts/install/install-arch.sh

# Debian / Ubuntu family
./scripts/install/install-debian.sh

# Fedora / RHEL family
./scripts/install/install-fedora.sh

# openSUSE family
./scripts/install/install-opensuse.sh
```
