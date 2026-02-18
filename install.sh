#!/usr/bin/env bash
# pdfshrinker installer
# Usage: curl -fsSL https://raw.githubusercontent.com/bzon/pdf-size-shrinker/main/install.sh | bash
set -euo pipefail

REPO="bzon/pdf-size-shrinker"
BINARY="pdfshrinker"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# ---------------------------------------------------------------------------
# Detect OS and architecture
# ---------------------------------------------------------------------------
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
  Darwin)
    case "$ARCH" in
      x86_64)         TARGET="x86_64-apple-darwin" ;;
      arm64 | aarch64) TARGET="aarch64-apple-darwin" ;;
      *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported Linux architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS. Download manually from https://github.com/$REPO/releases" >&2
    exit 1
    ;;
esac

# ---------------------------------------------------------------------------
# Resolve latest release tag
# ---------------------------------------------------------------------------
echo "Fetching latest release..."
VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' \
  | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
  echo "Could not determine latest version. Check https://github.com/$REPO/releases" >&2
  exit 1
fi

echo "Installing $BINARY $VERSION ($TARGET)..."

# ---------------------------------------------------------------------------
# Download and extract
# ---------------------------------------------------------------------------
URL="https://github.com/$REPO/releases/download/$VERSION/pdfshrinker-${VERSION}-${TARGET}.tar.gz"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/archive.tar.gz"
tar -xzf "$TMP/archive.tar.gz" -C "$TMP"

# ---------------------------------------------------------------------------
# Install binary
# ---------------------------------------------------------------------------
chmod +x "$TMP/$BINARY"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP/$BINARY" "$INSTALL_DIR/$BINARY"
else
  echo "Writing to $INSTALL_DIR requires elevated permissions..."
  sudo mv "$TMP/$BINARY" "$INSTALL_DIR/$BINARY"
fi

echo ""
echo "Installed $BINARY to $INSTALL_DIR/$BINARY"
echo "Run: $BINARY --help"
