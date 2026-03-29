#!/usr/bin/env bash
set -euo pipefail

REPO="tadasi/diagram-cli"
INSTALL_DIR="${DG_INSTALL_DIR:-/usr/local/bin}"

# --- detect platform ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin) os="apple-darwin" ;;
  Linux)  os="unknown-linux-gnu" ;;
  *)      echo "Error: unsupported OS: ${OS}" >&2; exit 1 ;;
esac

case "${ARCH}" in
  x86_64)       arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *)            echo "Error: unsupported architecture: ${ARCH}" >&2; exit 1 ;;
esac

TARGET="${arch}-${os}"
ASSET="dg-${TARGET}.tar.gz"

# --- resolve version ---
if [ -n "${DG_VERSION:-}" ]; then
  TAG="v${DG_VERSION#v}"
  URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"
else
  URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
fi

echo "Installing dg (${TARGET})..."

TMPDIR_DL="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_DL}"' EXIT

curl -fsSL "${URL}" -o "${TMPDIR_DL}/${ASSET}"
tar xzf "${TMPDIR_DL}/${ASSET}" -C "${TMPDIR_DL}"

if [ -w "${INSTALL_DIR}" ]; then
  mv "${TMPDIR_DL}/dg" "${INSTALL_DIR}/dg"
else
  echo "Installing to ${INSTALL_DIR} (requires sudo)..."
  sudo mv "${TMPDIR_DL}/dg" "${INSTALL_DIR}/dg"
fi

chmod +x "${INSTALL_DIR}/dg"

echo "Installed dg to ${INSTALL_DIR}/dg"
echo "Run 'dg init' to get started."
