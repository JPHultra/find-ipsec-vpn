# Maintainer : Christian Rebischke <Chris.Rebischke@archlinux.org>
# Contributor: João Gomes <joao.gomes@findmore.pt>

pkgname=strongswan-fortigate
pkgver=6.0.7
pkgrel=1
pkgdesc='Open source IPsec implementation - patched for FortiGate XAuth passcode/OTP challenges'
url='https://www.strongswan.org'
license=('GPL-2.0-only')
arch=('x86_64')
makedepends=('libnm' 'systemd' 'ruby' 'ruby-rdoc' 'mariadb' 'python-build' 'python-installer' 'python-setuptools' 'python-wheel')
depends=('curl' 'gmp' 'iproute2' 'openssl' 'sqlite' 'libcap' 'systemd-libs' 'pam')
provides=("strongswan=${pkgver}")
conflicts=('strongswan')
backup=(
  etc/ipsec.conf
  etc/ipsec.secrets
  etc/swanctl/swanctl.conf
  etc/strongswan.conf
  etc/strongswan.d/{charon-logging.conf,charon-nm.conf,charon-systemd.conf,charon.conf,pki.conf,pool.conf,starter.conf,swanctl.conf}
  etc/strongswan.d/charon/{aesni.conf,agent.conf,attr-sql.conf,attr.conf,bypass-lan.conf,chapoly.conf,cmac.conf,connmark.conf,constraints.conf,counters.conf,curl.conf,dhcp.conf,dnscert.conf,dnskey.conf,drbg.conf,eap-aka-3gpp2.conf,eap-aka.conf,eap-dynamic.conf,eap-gtc.conf,eap-identity.conf,eap-md5.conf,eap-mschapv2.conf,eap-peap.conf,eap-radius.conf,eap-sim-file.conf,eap-sim.conf,eap-simaka-pseudonym.conf,eap-simaka-reauth.conf,eap-tls.conf,eap-ttls.conf,ext-auth.conf,farp.conf,fips-prf.conf,forecast.conf,gmp.conf,ha.conf,kdf.conf,kernel-netlink.conf,ldap.conf,mgf1.conf,ml.conf,mysql.conf,nonce.conf,openssl.conf,pem.conf,pgp.conf,pkcs1.conf,pkcs11.conf,pkcs7.conf,pkcs8.conf,pubkey.conf,radattr.conf,random.conf,resolve.conf,revocation.conf,sha3.conf,socket-default.conf,sql.conf,sqlite.conf,sshkey.conf,stroke.conf,unity.conf,updown.conf,vici.conf,x509.conf,xauth-eap.conf,xauth-generic.conf,xauth-noauth.conf,xauth-pam.conf,xcbc.conf})
source=("https://download.strongswan.org/strongswan-${pkgver}.tar.bz2"{,.sig}
        "xauth-passcode.patch")
validpgpkeys=("948F158A4E76A27BF3D07532DF42C170B34DBA77")
sha512sums=('1d10b0eaf39072db1d7f5237661e71c81107bb0bdea6a4bcbdff0c54eb7f72d6487f78c91a6f527c69ff20f9ee611d7e8cedcec4c5cc65d53ecc4301e1353240'
            'SKIP'
            'SKIP')

prepare() {
  cd "strongswan-${pkgver}"
  patch -Np1 -i "${srcdir}/xauth-passcode.patch"
  sed -i 's/$(PYTHON) -m build/$(PYTHON) -m build --wheel --no-isolation/' src/libcharon/plugins/vici/python/Makefile.am
  autoreconf -fiv -I /usr/share/gettext/m4
}

build() {
  CFLAGS+=' -std=gnu17'
  cd "strongswan-${pkgver}"
  ./configure --prefix=/usr \
    --sbindir=/usr/bin \
    --sysconfdir=/etc \
    --libexecdir=/usr/lib \
    --with-ipsecdir=/usr/lib/strongswan \
    --with-nm-ca-dir=/etc/ssl/certs \
    --enable-integrity-test \
    --enable-sqlite \
    --enable-pkcs11 \
    --enable-openssl \
    --enable-curl \
    --enable-sql \
    --enable-attr-sql \
    --enable-farp \
    --enable-dhcp \
    --enable-eap-sim \
    --enable-eap-sim-file \
    --enable-eap-simaka-pseudonym \
    --enable-eap-simaka-reauth \
    --enable-eap-identity \
    --enable-eap-md5 \
    --enable-eap-gtc \
    --enable-eap-aka \
    --enable-eap-aka-3gpp2 \
    --enable-eap-mschapv2 \
    --enable-eap-radius \
    --enable-xauth-eap \
    --enable-ha \
    --enable-vici \
    --enable-swanctl \
    --enable-systemd \
    --enable-ext-auth \
    --enable-mysql \
    --enable-ldap \
    --enable-cmd \
    --enable-forecast \
    --enable-connmark \
    --enable-aesni \
    --enable-eap-ttls \
    --enable-radattr \
    --enable-xauth-pam \
    --enable-xauth-noauth \
    --enable-eap-dynamic \
    --enable-eap-peap \
    --enable-eap-tls \
    --enable-chapoly \
    --enable-unity \
    --with-capabilities=libcap \
    --enable-mgf1 \
    --enable-sha3 \
    --enable-dnscert \
    --enable-nm \
    --enable-agent \
    --enable-bypass-lan \
    --enable-ruby-gems \
    --enable-ruby-gems-install \
    --enable-python-wheels \
    --enable-ml \
    --enable-stroke
  make
}

package() {
  local _gemdir="$(gem env gemdir)"

  cd "strongswan-${pkgver}"
  make DESTDIR="${pkgdir}" install

  python -m installer --destdir="$pkgdir" src/libcharon/plugins/vici/python/dist/*.whl

  # remove unrepreducible files
  rm -r "${pkgdir}"/${_gemdir}/cache/
}
