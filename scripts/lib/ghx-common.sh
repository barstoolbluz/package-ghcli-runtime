#!/usr/bin/env bash
# ghx-common.sh — shared functions for ghx commands

GHX_VERSION="0.1.0"

die() {
    printf 'Error: %s\n' "$*" >&2
    exit 1
}

warn() {
    printf 'Warning: %s\n' "$*" >&2
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

need_cmd() {
    have_cmd "$1" || die "$1 is required but not found on PATH"
}

# --- Output formatting (gum-aware) ---

fmt_header() {
    if have_cmd gum; then
        gum style --bold --foreground 212 "$*"
    else
        printf '\033[1;35m%s\033[0m\n' "$*"
    fi
}

fmt_info() {
    if have_cmd gum; then
        gum style --foreground 45 "ℹ $*"
    else
        printf '\033[0;36mℹ %s\033[0m\n' "$*"
    fi
}

fmt_success() {
    if have_cmd gum; then
        gum style --foreground 76 "✓ $*"
    else
        printf '\033[0;32m✓ %s\033[0m\n' "$*"
    fi
}

fmt_error() {
    if have_cmd gum; then
        gum style --foreground 196 "✗ $*" >&2
    else
        printf '\033[0;31m✗ %s\033[0m\n' "$*" >&2
    fi
}

# --- Auth check ---

ensure_gh_auth() {
    if ! gh auth status >/dev/null 2>&1; then
        fmt_error "Not authenticated with GitHub"
        fmt_info "Run 'ghcli-runtime' to set up authentication, or 'gh auth login' directly."
        exit 1
    fi
}

# --- Learn system ---

show_learn() {
    local command="$1" subcommand="${2:-}"
    local learn_file

    if [[ -n "$subcommand" ]]; then
        learn_file="${GHX_ROOT}/lib/ghx/learn/${command}-${subcommand}.md"
    else
        learn_file="${GHX_ROOT}/lib/ghx/learn/${command}.md"
    fi

    if [[ ! -f "$learn_file" ]]; then
        die "No learn content found for '${command}${subcommand:+ $subcommand}'"
    fi

    if have_cmd gum; then
        gum format < "$learn_file"
    elif have_cmd less; then
        less -R "$learn_file"
    else
        cat "$learn_file"
    fi
}

# --- Dispatch ---

dispatch_subcommand() {
    local command="$1" subcommand="${2:-}"
    shift 2 || shift $#
    local args=("$@")
    local filtered_args=()
    local learn_mode=0

    # Extract --learn from args
    for arg in "${args[@]}"; do
        if [[ "$arg" == "--learn" ]]; then
            learn_mode=1
        else
            filtered_args+=("$arg")
        fi
    done

    if [[ "$learn_mode" -eq 1 ]]; then
        show_learn "$command" "$subcommand"
        return 0
    fi

    if [[ -z "$subcommand" || "$subcommand" == "--help" || "$subcommand" == "-h" ]]; then
        local help_fn="cmd_${command}_help"
        if declare -f "$help_fn" >/dev/null 2>&1; then
            "$help_fn"
        else
            die "Unknown command: $command"
        fi
        return 0
    fi

    if [[ "${filtered_args[0]:-}" == "--help" || "${filtered_args[0]:-}" == "-h" ]]; then
        local sub_help_fn="cmd_${command}_${subcommand}_help"
        if declare -f "$sub_help_fn" >/dev/null 2>&1; then
            "$sub_help_fn"
            return 0
        fi
    fi

    local fn="cmd_${command}_${subcommand}"
    if declare -f "$fn" >/dev/null 2>&1; then
        "$fn" "${filtered_args[@]}"
    else
        die "Unknown subcommand: $command $subcommand"
    fi
}

# --- Top-level help ---

ghx_show_help() {
    cat <<'EOF'
ghx — a friendly GitHub CLI wrapper with built-in learning

Usage: ghx <command> [subcommand] [options]

Commands:
  pr       Pull request workflow (create, list, view, checkout, merge)
  issue    Issue management (create, list, view)
  repo     Repository operations (clone, view, fork)
  run      CI/workflow monitoring (list, view, watch)
  status   Dashboard of your PRs, issues, and checks

Global options:
  --help       Show this help
  --version    Show version
  --learn      Show educational walkthrough for any subcommand

Examples:
  ghx pr create --draft -t "My feature"
  ghx pr create --learn          # learn about PR creation
  ghx issue list --label bug
  ghx status
EOF
}
