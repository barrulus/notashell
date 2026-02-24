# Maintainer: Vijay Papanaboina <https://github.com/Vijay-papanaboina>

pkgname=notashell-git
pkgver=1
pkgrel=1
pkgdesc="A lightweight system control panel for Wayland compositors"
arch=('x86_64')
url="https://github.com/barrulus/notashell"
license=('MIT')
depends=('gtk4' 'gtk4-layer-shell' 'networkmanager' 'bluez' 'libpulse' 'wayland' 'libxkbcommon')
makedepends=('rust' 'cargo' 'git')
provides=('notashell')
conflicts=('notashell')
source=("git+https://github.com/barrulus/notashell.git")
sha256sums=('SKIP')

pkgver() {
    cd "$srcdir/notashell"
    printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

build() {
    cd "$srcdir/notashell"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --release --locked --all-features
}

check() {
    cd "$srcdir/notashell"
    cargo test --release --locked
}

package() {
    cd "$srcdir/notashell"

    # Install binary
    install -Dm755 "target/release/notashell" "$pkgdir/usr/bin/notashell"

    # Install license
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Install README
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
