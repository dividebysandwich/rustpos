#!/bin/bash
set -e

# Usage: ./debian/build-printclient-deb.sh <arch>
# arch: amd64, arm64, armhf

ARCH="${1:-amd64}"
VERSION=$(grep '^version' printclient/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
PKG_NAME="rustpos-printclient"
PKG_DIR="${PKG_NAME}_${VERSION}_${ARCH}"

echo "Building ${PKG_NAME} ${VERSION} for ${ARCH}..."

# Clean and create package directory structure
rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/opt/rustpos-printclient"
mkdir -p "$PKG_DIR/lib/systemd/system"

# Determine binary source based on arch
case "$ARCH" in
    amd64)
        BIN_SRC="target/release/rustpos-printclient"
        ;;
    arm64)
        BIN_SRC="target/aarch64-unknown-linux-gnu/release/rustpos-printclient"
        ;;
    armhf)
        BIN_SRC="target/armv7-unknown-linux-gnueabihf/release/rustpos-printclient"
        ;;
    *)
        echo "Unknown architecture: $ARCH"
        exit 1
        ;;
esac

if [ ! -f "$BIN_SRC" ]; then
    echo "Binary not found at $BIN_SRC"
    echo "Build the print client first with: cargo build --release -p rustpos-printclient"
    exit 1
fi

# Copy binary
cp "$BIN_SRC" "$PKG_DIR/opt/rustpos-printclient/rustpos-printclient"
chmod 755 "$PKG_DIR/opt/rustpos-printclient/rustpos-printclient"

# Copy example config
cp printclient/printclient.toml "$PKG_DIR/opt/rustpos-printclient/printclient.toml.example"

# Copy systemd service
cp debian/rustpos-printclient.service "$PKG_DIR/lib/systemd/system/"

# Copy maintainer scripts
cp debian/printclient-postinst "$PKG_DIR/DEBIAN/postinst"
cp debian/printclient-prerm "$PKG_DIR/DEBIAN/prerm"
cp debian/printclient-postrm "$PKG_DIR/DEBIAN/postrm"
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
Depends: libc6, libgcc-s1, libssl3 | libssl1.1, libudev1
Description: RustPOS Remote Print Client
 A lightweight remote receipt printing client for the RustPOS point of sale
 system. Connects to a RustPOS server via WebSocket to receive and print
 receipts on a locally attached ESC/POS thermal printer.
Homepage: https://github.com/dividebysandwich/rustpos
EOF

# Build the .deb
dpkg-deb --build --root-owner-group "$PKG_DIR"

echo ""
echo "Package built: ${PKG_DIR}.deb"
ls -lh "${PKG_DIR}.deb"
