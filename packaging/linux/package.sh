#!/usr/bin/env bash
# Builds Linux release artifacts into dist/:
#   ram-altman_<version>_amd64.deb      (Debian/Ubuntu/Mint/Pop!_OS: double-click or `sudo apt install ./file.deb`)
#   ram-altman-<version>-linux-x86_64.tar.gz   (any distro: extract and run ./install.sh)
set -euo pipefail
cd "$(dirname "$0")/../.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
MAINTAINER="${DEB_MAINTAINER:-RAM Altman developers <noreply@example.com>}"
BIN=target/release/ram-altman

cargo build --release --locked
rm -rf dist && mkdir -p dist
# Minimum glibc = newest GLIBC_x.y symbol the binary links against.
GLIBC_MIN=$(objdump -T "$BIN" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)

# ---- .deb ----
PKG=dist/deb/ram-altman_${VERSION}_amd64
install -Dm755 "$BIN" "$PKG/usr/bin/ram-altman"
install -Dm644 packaging/linux/ram-altman.desktop "$PKG/usr/share/applications/ram-altman.desktop"
install -Dm644 assets/icon.png "$PKG/usr/share/icons/hicolor/256x256/apps/ram-altman.png"
install -d "$PKG/usr/share/doc/ram-altman"
cat > "$PKG/usr/share/doc/ram-altman/copyright" <<COPYRIGHT
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: ram-altman

Files: *
Copyright: $(date +%Y) RAM Altman contributors
License: GPL-3.0-or-later
 On Debian systems, the full text of the GNU General Public License
 version 3 can be found in /usr/share/common-licenses/GPL-3.
COPYRIGHT
chmod 644 "$PKG/usr/share/doc/ram-altman/copyright"
mkdir -p "$PKG/DEBIAN"
cat > "$PKG/DEBIAN/control" <<CONTROL
Package: ram-altman
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Maintainer: ${MAINTAINER}
Depends: libc6 (>= ${GLIBC_MIN}), libgl1, libegl1, libxkbcommon0
Recommends: libxkbcommon-x11-0, libwayland-client0, libwayland-egl1
Installed-Size: $(du -sk "$PKG/usr" | cut -f1)
Description: RAM Altman: Memory Is All You Need
 A small always-on-top desktop widget that shows available and total RAM
 next to a cartoon Sam Altman who gets angrier the more RAM you leave
 unused. He is only happy when at least 90% of your RAM is in use.
CONTROL
dpkg-deb --root-owner-group --build "$PKG" "dist/ram-altman_${VERSION}_amd64.deb"
rm -rf dist/deb

# ---- portable tarball ----
TAR=ram-altman-${VERSION}-linux-x86_64
mkdir -p "dist/$TAR"
cp "$BIN" packaging/linux/ram-altman.desktop packaging/linux/install.sh LICENSE "dist/$TAR/"
cp assets/icon.png "dist/$TAR/ram-altman.png"
chmod +x "dist/$TAR/install.sh" "dist/$TAR/ram-altman"
tar -C dist -czf "dist/$TAR.tar.gz" "$TAR"
rm -rf "dist/$TAR"

ls -lh dist
