#!/usr/bin/env bash
set -euo pipefail

REPO="GarrickZ2/claude-code-cleaner"
BIN="claude-code-cleaner"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "unknown-linux-gnu" ;;
        Darwin*) echo "apple-darwin" ;;
        *)       error "Unsupported OS: $(uname -s). Only Linux and macOS are supported." ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m). Only x86_64 and aarch64 are supported." ;;
    esac
}

# Get latest release tag
get_latest_version() {
    if command -v curl &>/dev/null; then
        curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

main() {
    local version="${1:-}"
    local os arch target

    os="$(detect_os)"
    arch="$(detect_arch)"
    target="${arch}-${os}"

    info "Detected platform: ${target}"

    if [ -z "$version" ]; then
        info "Fetching latest release..."
        version="$(get_latest_version)"
        if [ -z "$version" ]; then
            error "Could not determine latest version. Specify a version: $0 v0.1.0"
        fi
    fi

    info "Installing ${BIN} ${version}..."

    local archive="${BIN}-${version}-${target}.tar.gz"
    local url="https://github.com/${REPO}/releases/download/${version}/${archive}"

    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading ${url}..."
    if command -v curl &>/dev/null; then
        curl -fSL "$url" -o "${tmpdir}/${archive}" || error "Download failed. Check that version ${version} exists and has a binary for ${target}."
    else
        wget -q "$url" -O "${tmpdir}/${archive}" || error "Download failed. Check that version ${version} exists and has a binary for ${target}."
    fi

    info "Extracting..."
    tar xzf "${tmpdir}/${archive}" -C "$tmpdir"

    info "Installing to ${INSTALL_DIR}/${BIN}..."
    if [ -w "$INSTALL_DIR" ]; then
        cp "${tmpdir}/${BIN}-${version}-${target}/${BIN}" "${INSTALL_DIR}/${BIN}"
        chmod +x "${INSTALL_DIR}/${BIN}"
    else
        warn "Need sudo to install to ${INSTALL_DIR}"
        sudo cp "${tmpdir}/${BIN}-${version}-${target}/${BIN}" "${INSTALL_DIR}/${BIN}"
        sudo chmod +x "${INSTALL_DIR}/${BIN}"
    fi

    info "Successfully installed ${BIN} ${version} to ${INSTALL_DIR}/${BIN}"
    echo ""
    echo "  Run it with:  ${BIN}"
    echo ""
    echo "  Or specify a custom install directory:"
    echo "    INSTALL_DIR=~/.local/bin bash install.sh"
    echo ""
}

main "$@"
