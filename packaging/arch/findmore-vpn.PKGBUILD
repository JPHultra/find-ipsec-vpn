# Maintainer: João Gomes <joao.gomes@findmore.pt>
pkgname=findmore-vpn
pkgver=1.0.0
pkgrel=1
pkgdesc="Findmore FortiGate VPN Client GUI (Tauri + Rust PTY helper)"
arch=('x86_64')
url="https://vpn.findmore.pt"
license=('GPL-3.0-or-later')
depends=('strongswan-fortigate' 'gtk3' 'webkit2gtk-4.1' 'polkit' 'openssl')
makedepends=('cargo' 'npm' 'nodejs')
source=('findmore-vpn.desktop'
        'findmore-vpn.svg')
sha512sums=('SKIP' 'SKIP')

build() {
  # The PKGBUILD is located in packaging/arch
  # Go to the workspace root directory containing package.json (3 levels up from src)
  cd "${srcdir}/../../../"
  
  # Install node modules
  npm install
  
  # Compile the Tauri frontend and Rust backend in release mode
  npx tauri build --no-bundle
}

package() {
  # Go to the workspace root directory (3 levels up from src)
  cd "${srcdir}/../../../"
  
  # Install binaries to /usr/bin
  install -Dm755 "target/release/findmore-vpn-gui" "${pkgdir}/usr/bin/findmore-vpn-gui"
  install -Dm755 "target/release/findmore-vpn-helper" "${pkgdir}/usr/bin/findmore-vpn-helper"
  
  # Install Polkit policy XML
  install -Dm644 "packaging/polkit/pt.findmore.vpn.helper.policy" "${pkgdir}/usr/share/polkit-1/actions/pt.findmore.vpn.helper.policy"
  
  # Install Desktop entry shortcut
  install -Dm644 "packaging/arch/findmore-vpn.desktop" "${pkgdir}/usr/share/applications/findmore-vpn.desktop"
  
  # Install SVG application icon
  install -Dm644 "packaging/arch/findmore-vpn.svg" "${pkgdir}/usr/share/icons/hicolor/scalable/apps/findmore-vpn.svg"
}
