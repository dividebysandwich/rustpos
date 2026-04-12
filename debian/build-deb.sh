#!/bin/bash
set -e

# Usage: ./debian/build-deb.sh <arch>
# arch: amd64, arm64, armhf

ARCH="${1:-amd64}"
VERSION=$(grep '^version' frontend/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
PKG_NAME="rustpos"
PKG_DIR="${PKG_NAME}_${VERSION}_${ARCH}"

echo "Building ${PKG_NAME} ${VERSION} for ${ARCH}..."

# Clean and create package directory structure
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/opt/rustpos/site"
mkdir -p "$PKG_DIR/opt/rustpos/data/item_images"
mkdir -p "$PKG_DIR/lib/systemd/system"

# Determine binary source based on arch
case "$ARCH" in
    amd64)
        BIN_SRC="target/release/rustpos"
        ;;
    arm64)
        BIN_SRC="target/aarch64-unknown-linux-gnu/release/rustpos"
        ;;
    armhf)
        BIN_SRC="target/armv7-unknown-linux-gnueabihf/release/rustpos"
        ;;
    *)
        echo "Unknown architecture: $ARCH"
        exit 1
        ;;
esac

if [ ! -f "$BIN_SRC" ]; then
    echo "Binary not found at $BIN_SRC"
    echo "Build the project first with cargo-leptos"
    exit 1
fi

# Copy binary
cp "$BIN_SRC" "$PKG_DIR/opt/rustpos/rustpos"
chmod 755 "$PKG_DIR/opt/rustpos/rustpos"

# Copy static site assets
cp -r site/* "$PKG_DIR/opt/rustpos/site/"

# Copy systemd service
cp debian/rustpos.service "$PKG_DIR/lib/systemd/system/"

# Copy maintainer scripts
cp debian/postinst "$PKG_DIR/DEBIAN/"
cp debian/prerm "$PKG_DIR/DEBIAN/"
cp debian/postrm "$PKG_DIR/DEBIAN/"
chmod 755 "$PKG_DIR/DEBIAN/postinst"
chmod 755 "$PKG_DIR/DEBIAN/prerm"
chmod 755 "$PKG_DIR/DEBIAN/postrm"

# Calculate installed size in KB
INSTALLED_SIZE=$(du -sk "$PKG_DIR" | cut -f1)

# Generate control file
cat > "$PKG_DIR/DEBIAN/control" <<EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Architecture: ${ARCH}
Maintainer: RustPOS Contributors
Installed-Size: ${INSTALLED_SIZE}
Depends: libc6, libgcc-s1, libssl3 | libssl1.1
Description: RustPOS Point of Sale System
 A modern, touch-friendly point of sale system built with Rust.
 Features include sales management, kitchen display, receipt printing,
 stock tracking, user roles, and multi-language support.
Homepage: https://github.com/dividebysandwich/rustpos
EOF

# Build the .deb
dpkg-deb --build --root-owner-group "$PKG_DIR"

echo ""
echo "Package built: ${PKG_DIR}.deb"
ls -lh "${PKG_DIR}.deb"
