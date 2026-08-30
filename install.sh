#!/bin/sh
# invoka installer — installs the release binary from GitHub releases.
#
#   curl -fsSL https://raw.githubusercontent.com/danielmadu/invoka/master/install.sh | sh
#
# Environment overrides:
#   INSTALL_DIR  where to place the binary (default: ~/.local/bin)
#   VERSION      release tag to install (default: latest)
#   REPO         GitHub repo (default: danielmadu/invoka)

set -eu

REPO="${REPO:-danielmadu/invoka}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

main() {
    need curl; need tar

    case "$(uname -s)" in
        Linux) ;;
        *)
            err "unsupported OS: $(uname -s). invoka releases currently ship Linux binaries (Windows support planned)."
            ;;
    esac

    case "$(uname -m)" in
        x86_64)  arch="x86_64" ;;
        aarch64) arch="aarch64" ;;
        *)
            err "unsupported architecture: $(uname -m)"
            ;;
    esac

    version="${VERSION:-$(latest_tag)}"
    [ -n "$version" ] || err "could not determine the latest release; set VERSION=<tag> and retry"

    asset="invoka-${version}-${arch}-linux.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${asset}"

    info "downloading ${REPO} ${version} (${arch}-linux)"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    if ! curl -fsSL --retry 3 -o "${tmp}/${asset}" "$url"; then
        err "download failed: ${url}
       does the release ${version} ship an asset for ${arch}-linux?"
    fi

    tar -xzf "${tmp}/${asset}" -C "$tmp"

    # Accept both layouts: binary at archive root or nested inside a dir.
    binary="$(find "$tmp" -type f -name invoka -perm -u+x | head -n 1)"
    [ -n "$binary" ] || binary="$(find "$tmp" -type f -name 'invoka*' ! -name '*.tar.gz' | head -n 1)"
    [ -n "$binary" ] || err "no invoka binary found inside ${asset}"

    mkdir -p "$INSTALL_DIR"
    install -m 755 "$binary" "${INSTALL_DIR}/invoka"

    info "installed invoka to ${INSTALL_DIR}/invoka"
    "${INSTALL_DIR}/invoka" --version || true

    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            info "add ${INSTALL_DIR} to your PATH, e.g.:"
            info "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
            ;;
    esac
}

latest_tag() {
    # Resolve the latest release without the API (no rate limits, no jq):
    # /releases/latest 302-redirects to /tag/<version>; grab the Location
    # header directly instead of downloading the page.
    location="$(curl -fsS -D - -o /dev/null "https://github.com/${REPO}/releases/latest" 2>/dev/null \
        | tr -d '\r' | sed -n 's/^[Ll]ocation: //p' | head -n 1)"
    tag="${location##*/}"
    case "$tag" in
        v*) printf '%s' "$tag"; return ;;
    esac

    # Fallback: GitHub API.
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || err "missing dependency: $1"
}

info() {
    printf '[invoka] %s\n' "$1"
}

err() {
    printf '[invoka] error: %s\n' "$1" >&2
    exit 1
}

main "$@"
