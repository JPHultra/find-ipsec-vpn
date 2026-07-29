# Linux Distribution & Desktop Feature Compatibility Matrix

This document provides a detailed feature compatibility matrix, dependency mappings, and configuration guidelines for running Findmore FortiGate VPN across major Linux distributions and desktop environments.

---

## Feature Compatibility Matrix

| Distribution & Desktop Environment | PTY Tunnel Helper | Interactive 2FA OTP | System Tray Status & Traffic Dropdown | Desktop OS Notifications | Hardware/OS Keyring Storage | Auto-Reconnect on Drop | Autostart Support |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Arch Linux / Omarchy (Hyprland)** | ✅ Supported | ✅ Supported | ✅ Supported (Waybar SNI) | ✅ Supported (`mako`/`swaync`) | ✅ Supported (SecretService/0600) | ✅ Supported | ✅ Supported (`autostart.conf`) |
| **Ubuntu 22.04 / 24.04 (GNOME)** | ✅ Supported | ✅ Supported | ✅ Supported (AppIndicator) | ✅ Supported (GNOME Shell) | ✅ Supported (`libsecret`) | ✅ Supported | ✅ Supported (XDG Autostart) |
| **Fedora 39 / 40 / 41 (GNOME)** | ✅ Supported | ✅ Supported | ✅ Supported (AppIndicator) | ✅ Supported (GNOME Shell) | ✅ Supported (`libsecret`) | ✅ Supported | ✅ Supported (XDG Autostart) |
| **KDE Plasma 5 / 6 (Any Distro)** | ✅ Supported | ✅ Supported | ✅ Supported (Native KSNI) | ✅ Supported (KDE Notifications) | ✅ Supported (KWallet / SecretService) | ✅ Supported | ✅ Supported (XDG Autostart) |
| **Debian 12 / Linux Mint (XFCE/Cinnamon)** | ✅ Supported | ✅ Supported | ✅ Supported (Native Tray) | ✅ Supported (`xfce4-notifyd`) | ✅ Supported (`libsecret`) | ✅ Supported | ✅ Supported (XDG Autostart) |
| **openSUSE Leap / Tumbleweed** | ✅ Supported | ✅ Supported | ✅ Supported (Native Tray) | ✅ Supported (Desktop DBus) | ✅ Supported (`libsecret`) | ✅ Supported | ✅ Supported (XDG Autostart) |

---

## Distribution Package Dependencies

| Requirement / Subsystem | Arch Linux / Omarchy | Debian / Ubuntu / Mint / Pop!_OS | Fedora / RHEL / Rocky | openSUSE Leap / Tumbleweed |
| :--- | :--- | :--- | :--- | :--- |
| **IPsec Engine** | `strongswan-fortigate` | `strongswan` (`charon-cmd`) | `strongswan` | `strongswan` |
| **Polkit Elevation** | `polkit` | `policykit-1` | `polkit` | `polkit` |
| **GUI Web Engine** | `webkit2gtk-4.1` | `libwebkit2gtk-4.1-dev` | `webkit2gtk4.1-devel` | `webkit2gtk-4_1-devel` |
| **Secret Storage** | `libsecret` | `libsecret-1-dev` | `libsecret-devel` | `libsecret-devel` |
| **Cryptographic Library**| `openssl` | `libssl-dev` | `openssl-devel` | `libopenssl-devel` |
| **Package Installer** | `./scripts/install/install-arch.sh` | `./scripts/install/install-debian.sh` | `./scripts/install/install-fedora.sh` | `./scripts/install/install-opensuse.sh` |

---

## Distribution Gotchas & Desktop Environment Notes

### 1. GNOME Shell (Ubuntu & Fedora)
- **System Tray Icons**: GNOME Shell does not render legacy or SNI system tray icons out-of-the-box. Ubuntu ships with `gnome-shell-extension-appindicator` enabled by default. On stock Fedora GNOME, install the extension:
  ```bash
  sudo dnf install gnome-shell-extension-appindicator
  ```
- **Polkit Prompts**: GNOME uses `gnome-shell`'s built-in Polkit authentication agent.

### 2. Arch Linux / Omarchy (Hyprland & Waybar)
- **System Tray**: System tray indicators are rendered via Waybar's `tray` module using the StatusNotifierItem (SNI) protocol.
- **Autostart**: Uses `~/.config/hypr/autostart.conf` (`exec-once = findmore-vpn-gui --tray`).

### 3. Keyring Service Availability
- **SecretStorage API**: Findmore VPN communicates with `org.freedesktop.secrets` via `libsecret`. If running on a headless or bare window manager setup without Gnome Keyring or KWallet, credentials automatically fall back to `~/.config/findmore-vpn/secrets.json` protected with strict `0600` Linux user permissions.

### 4. AppArmor & SELinux Policy Notes
- **Fedora / RHEL (SELinux)**: Polkit execution for `/usr/bin/findmore-vpn-helper` via `pkexec` is pre-approved under standard `unconfined_service_t` or `unconfined_t` user domains.
- **Ubuntu / Debian (AppArmor)**: No custom AppArmor profiles interfere with PTY allocation or strongSwan execution.
