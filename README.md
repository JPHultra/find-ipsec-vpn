# Findmore FortiGate VPN GUI Client

A lightweight, secure, and beautiful desktop application for connecting to the Findmore FortiGate VPN on Arch Linux.

This application is built using **Tauri v2 (Rust + HTML/CSS/JS)**. It provides a privilege-separated frontend to drive strongSwan's `charon-cmd` engine, interactively handle FortiGate's email OTP passcode verification, and monitor live connection metrics.

---

## Architecture Design

```
┌──────────────────────────────────────┐
│       Findmore VPN GUI (User)        │
│  - Settings card & inputs            │
│  - Active connection metrics panel   │
│  - Secure OS Keyring credentials     │
└──────────────────┬───────────────────┘
                   │
                   │ stdin/stdout (JSON Lines)
                   ▼
┌──────────────────────────────────────┐
│   findmore-vpn-helper (Root/pkexec)  │
│  - Spawns charon-cmd inside POSIX PTY│
│  - Auto-fills PSK and password       │
│  - Intercepts and yields OTP prompt  │
│  - Parses Linux kernel XFRM stats    │
└──────────────────────────────────────┘
```

1. **Privilege Separation**: The GUI runs as a regular user. Tunnelling and routing operations are handled by an elevated child binary (`findmore-vpn-helper`) launched via `pkexec`.
2. **POSIX PTY spawning**: strongSwan prompts for keys via `/dev/tty`. Standard piped streams fail. The helper allocates a master/slave pseudo-terminal using `libc` so the credentials can be supplied programmatically without command line exposure.
3. **Interactive 2FA**: When FortiGate issues the `XAUTH_PASSCODE` challenge, the helper intercepts the `PIN:` prompt, signals the GUI, and pipes back the verification code submitted by the user.
4. **Traffic Statistics**: Real-time traffic stats (Bytes Sent/Received) are parsed directly from the kernel IPsec Security Association tables (`ip -s xfrm state`) every 1.5 seconds.

---

## Project Structure

```
findmore-vpn/
├── Cargo.toml                  # Workspace definition
├── package.json                # Frontend package configuration
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

## How to Build Locally

### 1. Requirements
Ensure the following tools are installed on your Arch Linux system:
* Node.js and NPM
* Rust (Cargo/rustc)
* base-devel, git

### 2. Install NPM dependencies
```bash
npm install
```

### 3. Build the project
To compile both the GUI and the helper in release mode:
```bash
npm run tauri build -- --no-bundle
```
This output is saved to the root `target/release/` directory:
- `target/release/findmore-vpn-gui`
- `target/release/findmore-vpn-helper`

---

## Arch Linux / Omarchy Installation

To package and install the application, simply run the installation script from the root of the repository:

```bash
./install.sh
```

The script will automatically check for GPG keys, resolve dependencies, build the custom patched strongSwan backend, compile the Tauri client, and register all Polkit and desktop shortcuts.

### Manual Installation (Alternative)

If you prefer to compile manually, follow these steps:

#### 1. Build and install the patched strongSwan
```bash
cd packaging/arch
makepkg -p strongswan-fortigate.PKGBUILD -si --noconfirm
```

#### 2. Build and install the Findmore VPN GUI
```bash
makepkg -p findmore-vpn.PKGBUILD -si --noconfirm
```

Once installed, the Findmore VPN client will be available in your application launcher menu. Clicking **Connect** will trigger a Polkit system prompt to authenticate administrative access.
