pkgname=findipsec-vpn
pkgver=1.0.0
pkgrel=1
pkgdesc="FindIPSec FortiGate VPN Client GUI (Tauri + Rust PTY helper)"
arch=('x86_64')
url="https://github.com/JPHultra/findmore-vpn"
license=('GPL-3.0-or-later')
depends=('strongswan-fortigate' 'gtk3' 'webkit2gtk-4.1' 'polkit' 'openssl')
makedepends=('cargo' 'npm' 'nodejs')
source=('findipsec-vpn.desktop'
        'findipsec-vpn.svg')
sha512sums=('SKIP' 'SKIP')

build() {
  cd "${srcdir}/../../../"
  npm install
  npx tauri build --no-bundle
}

package() {
  cd "${srcdir}/../../../"
  
  install -Dm755 "target/release/findipsec-vpn-gui" "${pkgdir}/usr/bin/findipsec-vpn-gui"
  install -Dm755 "target/release/findipsec-vpn-helper" "${pkgdir}/usr/bin/findipsec-vpn-helper"
  
  install -Dm644 "packaging/polkit/pt.findipsec.vpn.helper.policy" "${pkgdir}/usr/share/polkit-1/actions/pt.findipsec.vpn.helper.policy"
  install -Dm644 "packaging/arch/findipsec-vpn.desktop" "${pkgdir}/usr/share/applications/findipsec-vpn.desktop"
  install -Dm644 "packaging/arch/findipsec-vpn.svg" "${pkgdir}/usr/share/icons/hicolor/scalable/apps/findipsec-vpn.svg"
}
