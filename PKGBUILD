pkgname=acme_test_server
pkgver=1.0.0
pkgrel=1
arch=('x86_64')
depends=('systemd')
makedepends=('rust' 'cargo')
source=("git+https://github.com/hattmo/acme_test_server.git#branch=main")
sha256sums=('SKIP')

build() {
  cd "$srcdir/acme_test_server"
  cargo build --release --locked
}

package() {
  cd "$srcdir/acme_test_server"
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"

  # Install systemd service files
  install -Dm644 "foo.service" "$pkgdir/usr/lib/systemd/system/foo.service"
  install -Dm644 "bar.service" "$pkgdir/usr/lib/systemd/system/bar.service"
}

