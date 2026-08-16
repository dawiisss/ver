#!/usr/bin/env bash
# ==============================================================================
# VER (Very Easy Remote) - Uninstallation Script
# https://github.com/dawiisss/ver
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dawiisss/ver/main/uninstall.sh | bash
#   or:
#   ./uninstall.sh [OPTIONS]
# ==============================================================================

set -eo pipefail

# --- Configuration & Constants ---
APP_NAME="ver"
DESKTOP_ID="com.example.ver"

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
${COLOR_BOLD}VER (Very Easy Remote) Uninstaller${COLOR_RESET}

${COLOR_BOLD}USAGE:${COLOR_RESET}
    ./uninstall.sh [OPTIONS]
    curl -fsSL https://raw.githubusercontent.com/dawiisss/ver/main/uninstall.sh | bash -s -- [OPTIONS]

${COLOR_BOLD}OPTIONS:${COLOR_RESET}
    ${COLOR_CYAN}-u, --user${COLOR_RESET}             Remove from user space (~/.local)
    ${COLOR_CYAN}-s, --system${COLOR_RESET}           Remove from system directories (/usr/local, /usr)
    ${COLOR_CYAN}-p, --prefix <DIR>${COLOR_RESET}     Remove from custom prefix directory
    ${COLOR_CYAN}--purge${COLOR_RESET}                Also delete user configuration & connection library (~/.config/ver)
    ${COLOR_CYAN}-n, --dry-run${COLOR_RESET}          Simulate uninstallation actions without deleting files
    ${COLOR_CYAN}-h, --help${COLOR_RESET}             Show this help message

${COLOR_BOLD}EXAMPLES:${COLOR_RESET}
    ./uninstall.sh
    ./uninstall.sh --user
    sudo ./uninstall.sh --system
    ./uninstall.sh --purge
    ./uninstall.sh --dry-run

EOF
}

# --- Parse Arguments ---
TARGET_PREFIX=""
TARGET_MODE=""
PURGE=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -u|--user)
            TARGET_MODE="user"
            shift
            ;;
        -s|--system)
            TARGET_MODE="system"
            shift
            ;;
        -p|--prefix)
            if [ -z "${2:-}" ] || [[ "$2" == -* ]]; then
                abort "Option --prefix requires a directory path."
            fi
            TARGET_PREFIX="$2"
            shift 2
            ;;
        --purge)
            PURGE=true
            shift
            ;;
        -n|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            abort "Unknown option: $1 (run './uninstall.sh --help' for usage)"
            ;;
    esac
done

# --- Determine Target Prefixes ---
determine_prefixes() {
    PREFIXES=()

    if [ -n "$TARGET_PREFIX" ]; then
        PREFIXES+=("$TARGET_PREFIX")
    elif [ "$TARGET_MODE" = "user" ]; then
        PREFIXES+=("${XDG_DATA_HOME:-$HOME/.local}")
    elif [ "$TARGET_MODE" = "system" ]; then
        PREFIXES+=("/usr/local" "/usr")
    else
        # Auto-detect across standard locations
        local user_p="${XDG_DATA_HOME:-$HOME/.local}"
        local candidates=("$user_p" "/usr/local" "/usr")
        for p in "${candidates[@]}"; do
            if [ -f "$p/bin/${APP_NAME}" ] || [ -f "$p/share/applications/${DESKTOP_ID}.desktop" ]; then
                PREFIXES+=("$p")
            fi
        done

        # If nothing found by probing, default to user space and /usr/local
        if [ ${#PREFIXES[@]} -eq 0 ]; then
            PREFIXES=("$user_p" "/usr/local")
        fi
    fi
}

# --- Main Uninstallation Logic ---
do_uninstall() {
    determine_prefixes

    info "Scanning for VER installation files..."

    local files_to_remove=()
    local apps_dirs_to_update=()
    local icon_dirs_to_update=()

    for p in "${PREFIXES[@]}"; do
        local bin_file="$p/bin/${APP_NAME}"
        local desktop_file="$p/share/applications/${DESKTOP_ID}.desktop"
        local pixmap_file="$p/share/pixmaps/${DESKTOP_ID}.png"
        local icon_svg="$p/share/icons/hicolor/scalable/apps/${DESKTOP_ID}.svg"
        local icon_png="$p/share/icons/hicolor/256x256/apps/${DESKTOP_ID}.png"

        [ -f "$bin_file" ] && files_to_remove+=("$bin_file")
        if [ -f "$desktop_file" ]; then
            files_to_remove+=("$desktop_file")
            apps_dirs_to_update+=("$p/share/applications")
        fi
        [ -f "$pixmap_file" ] && files_to_remove+=("$pixmap_file")
        if [ -f "$icon_svg" ] || [ -f "$icon_png" ]; then
            [ -f "$icon_svg" ] && files_to_remove+=("$icon_svg")
            [ -f "$icon_png" ] && files_to_remove+=("$icon_png")
            icon_dirs_to_update+=("$p/share/icons/hicolor")
        fi
    done

    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/ver"
    local data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/ver"
    local cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/ver"
    local dirs_to_purge=()

    if [ "$PURGE" = true ]; then
        [ -d "$config_dir" ] && dirs_to_purge+=("$config_dir")
        [ -d "$data_dir" ] && dirs_to_purge+=("$data_dir")
        [ -d "$cache_dir" ] && dirs_to_purge+=("$cache_dir")
    fi

    if [ ${#files_to_remove[@]} -eq 0 ] && [ ${#dirs_to_purge[@]} -eq 0 ]; then
        warn "No installed VER files or components were found."
        return
    fi

    # Dry run preview
    if [ "$DRY_RUN" = true ]; then
        printf "\n${COLOR_YELLOW}${COLOR_BOLD}[DRY RUN] The following files/directories would be removed:${COLOR_RESET}\n"
        for f in "${files_to_remove[@]}"; do
            printf "  - %s\n" "$f"
        done
        for d in "${dirs_to_purge[@]}"; do
            printf "  - [PURGE DIR] %s\n" "$d"
        done
        printf "\nDry run complete. No files were removed.\n"
        return
    fi

    # Perform removal
    local removed_count=0
    for f in "${files_to_remove[@]}"; do
        info "Removing ${COLOR_CYAN}${f}${COLOR_RESET}..."
        if rm -f "$f" 2>/dev/null; then
            removed_count=$((removed_count + 1))
        else
            warn "Could not remove '$f' (permission denied? Try running with sudo)."
        fi
    done

    if [ "$PURGE" = true ]; then
        for d in "${dirs_to_purge[@]}"; do
            info "Purging directory ${COLOR_YELLOW}${d}${COLOR_RESET}..."
            rm -rf "$d" 2>/dev/null || warn "Could not purge '$d' (permission denied?)."
        done
    fi

    # Refresh desktop database & icon caches
    if command -v update-desktop-database >/dev/null 2>&1; then
        for ad in "${apps_dirs_to_update[@]}"; do
            if [ -d "$ad" ]; then
                update-desktop-database "$ad" 2>/dev/null || true
            fi
        done
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        for ic in "${icon_dirs_to_update[@]}"; do
            if [ -d "$ic" ]; then
                gtk-update-icon-cache -f -q -t "$ic" 2>/dev/null || true
            fi
        done
    fi

    printf "\n"
    success "${COLOR_BOLD}VER uninstallation completed! (${removed_count} files removed)${COLOR_RESET}"
    if [ "$PURGE" = false ] && [ -d "$config_dir" ]; then
        printf "  ${COLOR_DIM}Note: Your configuration & connections library at '${config_dir}' was preserved.${COLOR_RESET}\n"
        printf "  ${COLOR_DIM}To completely wipe configuration data, re-run with: ./uninstall.sh --purge${COLOR_RESET}\n"
    fi
    printf "\n"
}

# --- Execution ---
main() {
    do_uninstall
}

main "$@"
