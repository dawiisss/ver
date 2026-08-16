#!/usr/bin/env bash
# ==============================================================================
# VER (Very Easy Remote) - Installation Script
# https://github.com/dawiisss/ver
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dawiisss/ver/main/install.sh | bash
#   or:
#   ./install.sh [OPTIONS]
# ==============================================================================

set -eo pipefail

# --- Configuration & Constants ---
REPO="dawiisss/ver"
APP_NAME="ver"
DESKTOP_ID="com.example.ver"
DEFAULT_GITHUB_API="https://api.github.com/repos/${REPO}"
DEFAULT_GITHUB_URL="https://github.com/${REPO}"

# --- Formatting Helpers ---
if [ -t 1 ]; then
    COLOR_RESET="\033[0m"
    COLOR_BOLD="\033[1m"
    COLOR_GREEN="\033[32m"
    COLOR_BLUE="\033[34m"
    COLOR_CYAN="\033[36m"
    COLOR_YELLOW="\033[33m"
    COLOR_RED="\033[31m"
    COLOR_DIM="\033[2m"
else
    COLOR_RESET=""
    COLOR_BOLD=""
    COLOR_GREEN=""
    COLOR_BLUE=""
    COLOR_CYAN=""
    COLOR_YELLOW=""
    COLOR_RED=""
    COLOR_DIM=""
fi

info() {
    printf "${COLOR_BLUE}${COLOR_BOLD}==>${COLOR_RESET} %s\n" "$*"
}

success() {
    printf "${COLOR_GREEN}${COLOR_BOLD}==>${COLOR_RESET} %s\n" "$*"
}

warn() {
    printf "${COLOR_YELLOW}${COLOR_BOLD}WARNING:${COLOR_RESET} %s\n" "$*" >&2
}

error() {
    printf "${COLOR_RED}${COLOR_BOLD}ERROR:${COLOR_RESET} %s\n" "$*" >&2
}

abort() {
    error "$*"
    exit 1
}

# --- Help Text ---
show_help() {
    cat << EOF
${COLOR_BOLD}VER (Very Easy Remote) Installer${COLOR_RESET}

${COLOR_BOLD}USAGE:${COLOR_RESET}
    ./install.sh [OPTIONS]
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash -s -- [OPTIONS]

${COLOR_BOLD}OPTIONS:${COLOR_RESET}
    ${COLOR_CYAN}-u, --user${COLOR_RESET}             Install to user space (~/.local) [Default for non-root users]
    ${COLOR_CYAN}-s, --system${COLOR_RESET}           Install system-wide (/usr/local) [Default when run with sudo/root]
    ${COLOR_CYAN}-p, --prefix <DIR>${COLOR_RESET}     Specify custom installation prefix directory
    ${COLOR_CYAN}-v, --version <TAG>${COLOR_RESET}    Install a specific release version (e.g. 'v1.3.0' or '1.3.0')
    ${COLOR_CYAN}-l, --local${COLOR_RESET}            Install from local repository build (target/release/ver)
    ${COLOR_CYAN}-n, --dry-run${COLOR_RESET}          Simulate installation actions without making changes
    ${COLOR_CYAN}--no-deps-check${COLOR_RESET}        Skip checking for optional runtime dependencies
    ${COLOR_CYAN}-h, --help${COLOR_RESET}             Show this help message

${COLOR_BOLD}ENVIRONMENT VARIABLES:${COLOR_RESET}
    ${COLOR_CYAN}VERSION${COLOR_RESET}                Target release version to download (e.g. VERSION=v1.3.0)
    ${COLOR_CYAN}PREFIX${COLOR_RESET}                 Target installation prefix (e.g. PREFIX=/opt/ver)
    ${COLOR_CYAN}INSTALL_MODE${COLOR_RESET}           'user' or 'system'

${COLOR_BOLD}EXAMPLES:${COLOR_RESET}
    curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash
    ./install.sh --user
    sudo ./install.sh --system
    ./install.sh --version v1.3.0
    ./install.sh --local

EOF
}

# --- Parse Arguments ---
TARGET_VERSION="${VERSION:-latest}"
CUSTOM_PREFIX="${PREFIX:-}"
INSTALL_MODE="${INSTALL_MODE:-}"
LOCAL_MODE=false
DRY_RUN=false
CHECK_DEPS=true

while [[ $# -gt 0 ]]; do
    case "$1" in
        -u|--user)
            INSTALL_MODE="user"
            shift
            ;;
        -s|--system)
            INSTALL_MODE="system"
            shift
            ;;
        -p|--prefix)
            if [ -z "${2:-}" ] || [[ "$2" == -* ]]; then
                abort "Option --prefix requires a non-empty directory argument."
            fi
            CUSTOM_PREFIX="$2"
            shift 2
            ;;
        -v|--version)
            if [ -z "${2:-}" ] || [[ "$2" == -* ]]; then
                abort "Option --version requires a version tag argument."
            fi
            TARGET_VERSION="$2"
            shift 2
            ;;
        -l|--local)
            LOCAL_MODE=true
            shift
            ;;
        -n|--dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-deps-check)
            CHECK_DEPS=false
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            abort "Unknown option: $1 (run './install.sh --help' for usage)"
            ;;
    esac
done

# --- Platform & Architecture Detection ---
detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    if [ "$os" != "Linux" ]; then
        abort "VER currently supports Linux only (detected OS: $os)."
    fi

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        *)
            abort "Unsupported architecture: $arch. Pre-built releases are currently available for x86_64/amd64. To build from source for your architecture, see the README."
            ;;
    esac
}

# --- HTTP Client Detection ---
detect_http_client() {
    if command -v curl >/dev/null 2>&1; then
        HTTP_CLIENT="curl"
    elif command -v wget >/dev/null 2>&1; then
        HTTP_CLIENT="wget"
    else
        abort "Neither 'curl' nor 'wget' was found. Please install either curl or wget to download releases."
    fi
}

http_get() {
    local url="$1"
    if [ "$HTTP_CLIENT" = "curl" ]; then
        curl -fsSL -H "User-Agent: ver-installer" "$url"
    else
        wget -qO- --header="User-Agent: ver-installer" "$url"
    fi
}

http_download() {
    local url="$1"
    local output="$2"
    if [ "$HTTP_CLIENT" = "curl" ]; then
        curl -fSL --progress-bar -H "User-Agent: ver-installer" "$url" -o "$output"
    else
        wget --show-progress -q --header="User-Agent: ver-installer" "$url" -O "$output"
    fi
}

# --- Resolve Target Version & Release URL ---
resolve_version() {
    if [ "$LOCAL_MODE" = true ]; then
        return
    fi

    info "Resolving release for ${COLOR_CYAN}${REPO}${COLOR_RESET}..."

    if [ "$TARGET_VERSION" = "latest" ]; then
        # 1. Try GitHub API
        local api_response=""
        api_response=$(http_get "${DEFAULT_GITHUB_API}/releases/latest" 2>/dev/null || true)

        if [ -n "$api_response" ]; then
            TAG_NAME=$(printf "%s" "$api_response" | sed -n 's/.*"tag_name": *"*\([^",]*\)".*/\1/p' | head -n 1)
        fi

        # 2. Fallback: Query redirect URL if API was rate-limited or failed
        if [ -z "$TAG_NAME" ]; then
            if [ "$HTTP_CLIENT" = "curl" ]; then
                local final_url
                final_url=$(curl -sIL -o /dev/null -w '%{url_effective}' "${DEFAULT_GITHUB_URL}/releases/latest" 2>/dev/null || true)
                TAG_NAME=$(basename "$final_url" 2>/dev/null || true)
            elif [ "$HTTP_CLIENT" = "wget" ]; then
                local redir_url
                redir_url=$(wget --spider -S "${DEFAULT_GITHUB_URL}/releases/latest" 2>&1 | grep -i 'Location:' | tail -n 1 | awk '{print $2}' || true)
                TAG_NAME=$(basename "$redir_url" 2>/dev/null || true)
            fi
        fi

        if [ -z "$TAG_NAME" ] || [ "$TAG_NAME" = "latest" ]; then
            abort "Failed to detect the latest release version from GitHub. Please specify a version manually with '--version <tag>'."
        fi
    else
        TAG_NAME="$TARGET_VERSION"
    fi

    # Normalize tag name
    RELEASE_VERSION="${TAG_NAME#v}"
    if [[ "$TAG_NAME" != v* ]] && [[ "$TAG_NAME" =~ ^[0-9] ]]; then
        TAG_WITH_V="v${TAG_NAME}"
    else
        TAG_WITH_V="$TAG_NAME"
    fi

    info "Target release version: ${COLOR_GREEN}${TAG_NAME}${COLOR_RESET}"
}

# --- Determine Installation Paths ---
resolve_paths() {
    local is_root=false
    if [ "$(id -u)" -eq 0 ]; then
        is_root=true
    fi

    if [ -n "$CUSTOM_PREFIX" ]; then
        INSTALL_PREFIX="$CUSTOM_PREFIX"
    elif [ "$INSTALL_MODE" = "system" ]; then
        INSTALL_PREFIX="/usr/local"
    elif [ "$INSTALL_MODE" = "user" ]; then
        INSTALL_PREFIX="${XDG_DATA_HOME:-$HOME/.local}"
    elif [ "$is_root" = true ]; then
        INSTALL_PREFIX="/usr/local"
        INSTALL_MODE="system"
    else
        INSTALL_PREFIX="${HOME}/.local"
        INSTALL_MODE="user"
    fi

    # Ensure clean path resolution
    BIN_DIR="${INSTALL_PREFIX}/bin"
    APPS_DIR="${INSTALL_PREFIX}/share/applications"
    PIXMAPS_DIR="${INSTALL_PREFIX}/share/pixmaps"
    ICONS_SCALABLE_DIR="${INSTALL_PREFIX}/share/icons/hicolor/scalable/apps"
    ICONS_256_DIR="${INSTALL_PREFIX}/share/icons/hicolor/256x256/apps"

    info "Installation scope: ${COLOR_CYAN}${INSTALL_MODE:-custom}${COLOR_RESET}"
    info "Installation prefix: ${COLOR_CYAN}${INSTALL_PREFIX}${COLOR_RESET}"
    info "Binary destination: ${COLOR_CYAN}${BIN_DIR}/${APP_NAME}${COLOR_RESET}"
}

# --- Check Runtime & Protocol Dependencies ---
check_dependencies() {
    if [ "$CHECK_DEPS" = false ]; then
        return
    fi

    info "Checking environment & protocol dependencies..."

    local missing_protocols=()

    if ! command -v xfreerdp3 >/dev/null 2>&1 && ! command -v xfreerdp >/dev/null 2>&1; then
        missing_protocols+=("RDP / XRDP backend ('freerdp3' or 'freerdp3-x11' providing 'xfreerdp3')")
    fi

    if ! command -v vncviewer >/dev/null 2>&1; then
        missing_protocols+=("VNC backend ('tigervnc' or 'tigervnc-viewer' providing 'vncviewer')")
    fi

    if ! command -v remote-viewer >/dev/null 2>&1; then
        missing_protocols+=("SPICE backend ('virt-viewer' providing 'remote-viewer')")
    fi

    if [ ${#missing_protocols[@]} -gt 0 ]; then
        warn "The following optional remote client backends are not currently detected in PATH:"
        for item in "${missing_protocols[@]}"; do
            printf "     ${COLOR_DIM}-${COLOR_RESET} %s\n" "$item"
        done
        printf "     ${COLOR_DIM}Note: You can install these packages at any time via your distribution's package manager.${COLOR_RESET}\n\n"
    else
        success "All standard remote client backends (RDP, VNC, SPICE) detected!"
    fi
}

# --- Main Installation Logic ---
do_install() {
    local tmp_dir=""
    local src_bin=""
    local src_desktop=""
    local src_png=""
    local src_svg=""

    if [ "$DRY_RUN" = true ]; then
        local simulated_tarball="ver-linux-x86_64-${TAG_NAME:-latest}.tar.gz"
        local simulated_url="${DEFAULT_GITHUB_URL}/releases/download/${TAG_NAME:-latest}/${simulated_tarball}"

        printf "\n${COLOR_YELLOW}${COLOR_BOLD}[DRY RUN] The following actions would be performed:${COLOR_RESET}\n"
        if [ "$LOCAL_MODE" = true ]; then
            printf "  - Source: Local workspace build (target/release/ver, data/)\n"
        else
            printf "  - Download release bundle from: %s\n" "$simulated_url"
        fi
        printf "  - Create directories:\n"
        printf "      %s\n" "$BIN_DIR" "$APPS_DIR" "$PIXMAPS_DIR" "$ICONS_256_DIR" "$ICONS_SCALABLE_DIR"
        printf "  - Install binary:           -> %s\n" "${BIN_DIR}/${APP_NAME}"
        printf "  - Install desktop launcher: -> %s\n" "${APPS_DIR}/${DESKTOP_ID}.desktop"
        printf "  - Install icon (pixmaps):   -> %s\n" "${PIXMAPS_DIR}/${DESKTOP_ID}.png"
        printf "  - Install icon (256x256):   -> %s\n" "${ICONS_256_DIR}/${DESKTOP_ID}.png"
        printf "  - Install icon (scalable):  -> %s\n" "${ICONS_SCALABLE_DIR}/${DESKTOP_ID}.svg"
        printf "  - Update desktop and icon caches\n"
        printf "\nDry run complete. No changes were made.\n"
        return
    fi

    if [ "$LOCAL_MODE" = true ]; then
        info "Installing from local workspace..."
        local script_dir
        script_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" >/dev/null 2>&1 && pwd || pwd)"

        if [ -f "${script_dir}/target/release/ver" ]; then
            src_bin="${script_dir}/target/release/ver"
        elif [ -f "./target/release/ver" ]; then
            src_bin="./target/release/ver"
        elif [ -f "./ver" ]; then
            src_bin="./ver"
        else
            abort "Could not find local compiled binary at 'target/release/ver' or './ver'. Run 'cargo build --release' first or omit '--local' to download from GitHub Releases."
        fi

        src_desktop="${script_dir}/data/${DESKTOP_ID}.desktop"
        if [ ! -f "$src_desktop" ]; then
            src_desktop="./data/${DESKTOP_ID}.desktop"
        fi

        src_png="${script_dir}/data/${DESKTOP_ID}.png"
        if [ ! -f "$src_png" ]; then
            src_png="./data/${DESKTOP_ID}.png"
        fi

        src_svg="${script_dir}/data/${DESKTOP_ID}.svg"
        if [ ! -f "$src_svg" ]; then
            src_svg="./data/${DESKTOP_ID}.svg"
        fi
    else
        tmp_dir=$(mktemp -d -t ver-install-XXXXXX)
        trap 'rm -rf "$tmp_dir"' EXIT INT TERM

        local tarball_name="ver-linux-x86_64-${TAG_NAME}.tar.gz"
        local download_url="${DEFAULT_GITHUB_URL}/releases/download/${TAG_NAME}/${tarball_name}"
        local archive_path="${tmp_dir}/${tarball_name}"

        info "Downloading ${COLOR_CYAN}${tarball_name}${COLOR_RESET}..."
        
        # Try downloading with exact tag name, fallback to tag with or without 'v' if 404
        if ! http_download "$download_url" "$archive_path" 2>/dev/null; then
            local alt_tag=""
            if [[ "$TAG_NAME" == v* ]]; then
                alt_tag="${TAG_NAME#v}"
            else
                alt_tag="v${TAG_NAME}"
            fi
            local alt_tarball="ver-linux-x86_64-${alt_tag}.tar.gz"
            local alt_url="${DEFAULT_GITHUB_URL}/releases/download/${TAG_NAME}/${alt_tarball}"
            
            info "Trying alternative archive name: ${alt_tarball}..."
            if ! http_download "$alt_url" "$archive_path" 2>/dev/null; then
                # Also try matching release version tag
                local alt_url2="${DEFAULT_GITHUB_URL}/releases/download/${alt_tag}/${alt_tarball}"
                if ! http_download "$alt_url2" "$archive_path" 2>/dev/null; then
                    abort "Failed to download release archive from GitHub. URL tried: $download_url"
                fi
            fi
        fi

        info "Extracting release bundle..."
        tar -xzf "$archive_path" -C "$tmp_dir"

        src_bin="${tmp_dir}/ver"
        src_desktop="${tmp_dir}/${DESKTOP_ID}.desktop"
        src_png="${tmp_dir}/${DESKTOP_ID}.png"
        src_svg="${tmp_dir}/${DESKTOP_ID}.svg"

        if [ ! -f "$src_bin" ]; then
            # Search if nested inside extracted folder
            src_bin=$(find "$tmp_dir" -type f -name "ver" -perm -111 | head -n 1 || true)
        fi
        if [ ! -f "$src_desktop" ]; then
            src_desktop=$(find "$tmp_dir" -type f -name "${DESKTOP_ID}.desktop" | head -n 1 || true)
        fi
        if [ ! -f "$src_png" ]; then
            src_png=$(find "$tmp_dir" -type f -name "${DESKTOP_ID}.png" | head -n 1 || true)
        fi
        if [ ! -f "$src_svg" ]; then
            src_svg=$(find "$tmp_dir" -type f -name "${DESKTOP_ID}.svg" | head -n 1 || true)
        fi

        if [ ! -f "$src_bin" ]; then
            abort "Release archive did not contain executable 'ver' binary."
        fi
    fi


    # Create target directories
    mkdir -p "$BIN_DIR" "$APPS_DIR" "$PIXMAPS_DIR" "$ICONS_256_DIR" "$ICONS_SCALABLE_DIR"

    # Install binary
    info "Installing binary to ${COLOR_CYAN}${BIN_DIR}/${APP_NAME}${COLOR_RESET}..."
    install -m 755 "$src_bin" "${BIN_DIR}/${APP_NAME}"

    # Install desktop entry
    if [ -f "$src_desktop" ]; then
        info "Installing desktop entry to ${COLOR_CYAN}${APPS_DIR}/${DESKTOP_ID}.desktop${COLOR_RESET}..."
        install -m 644 "$src_desktop" "${APPS_DIR}/${DESKTOP_ID}.desktop"
    fi

    # Install icons
    if [ -f "$src_png" ]; then
        info "Installing icons..."
        install -m 644 "$src_png" "${PIXMAPS_DIR}/${DESKTOP_ID}.png"
        install -m 644 "$src_png" "${ICONS_256_DIR}/${DESKTOP_ID}.png"
    fi

    if [ -f "$src_svg" ]; then
        install -m 644 "$src_svg" "${ICONS_SCALABLE_DIR}/${DESKTOP_ID}.svg"
    fi

    # Refresh desktop database & icon caches
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -q -t "${INSTALL_PREFIX}/share/icons/hicolor" 2>/dev/null || true
    fi

    # PATH Check
    local in_path=false
    IFS=':' read -ra PATH_DIRS <<< "$PATH"
    for dir in "${PATH_DIRS[@]}"; do
        if [ "$dir" = "$BIN_DIR" ]; then
            in_path=true
            break
        fi
    done

    printf "\n"
    success "${COLOR_BOLD}VER (Very Easy Remote) successfully installed!${COLOR_RESET}"
    printf "\n"
    printf "  ${COLOR_BOLD}Installed Binary:${COLOR_RESET}   %s\n" "${BIN_DIR}/${APP_NAME}"
    printf "  ${COLOR_BOLD}Desktop Launcher:${COLOR_RESET}   %s\n" "${APPS_DIR}/${DESKTOP_ID}.desktop"
    printf "\n"

    if [ "$in_path" = false ]; then
        warn "'${BIN_DIR}' is not in your current PATH."
        printf "To launch 'ver' from anywhere in your terminal, add it to your shell configuration:\n\n"
        printf "    ${COLOR_BOLD}export PATH=\"%s:\$PATH\"${COLOR_RESET}\n\n" "$BIN_DIR"
        printf "Add the line above to your ${COLOR_CYAN}~/.bashrc${COLOR_RESET} or ${COLOR_CYAN}~/.zshrc${COLOR_RESET}, then restart your terminal.\n\n"
    else
        printf "  Launch immediately with:  ${COLOR_GREEN}${COLOR_BOLD}ver${COLOR_RESET}\n"
        printf "  Or find ${COLOR_BOLD}VER${COLOR_RESET} in your desktop application launcher.\n\n"
    fi
}

# --- Execution ---
main() {
    detect_platform
    detect_http_client
    resolve_version
    resolve_paths
    check_dependencies
    do_install
}

main "$@"
