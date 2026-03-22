#!/usr/bin/env bash
# ghx repo — repository operations

cmd_repo_help() {
    cat <<'EOF'
ghx repo — repository operations

Subcommands:
  clone      Clone a repository
  view       View repository details
  fork       Fork a repository

Use 'ghx repo <subcommand> --help' for details.
Add '--learn' to any subcommand for an educational walkthrough.
EOF
}

# --- repo clone ---

cmd_repo_clone_help() {
    cat <<'EOF'
ghx repo clone — clone a repository

Usage: ghx repo clone <REPO> [DIRECTORY] [options]

Arguments:
  REPO         owner/name or URL
  DIRECTORY    Target directory (optional)

Options:
  --learn      Learn about cloning repos
EOF
}

cmd_repo_clone() {
    ensure_gh_auth
    gh repo clone "$@"
}

# --- repo view ---

cmd_repo_view_help() {
    cat <<'EOF'
ghx repo view — view repository details

Usage: ghx repo view [REPO] [options]

Options:
  -w, --web         Open in browser
  --json FIELDS     Output as JSON
  --learn           Learn about viewing repos
EOF
}

cmd_repo_view() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--web)  args+=(--web); shift ;;
            --json)    args+=(--json "$2"); shift 2 ;;
            *)         args+=("$1"); shift ;;
        esac
    done
    gh repo view "${args[@]}"
}

# --- repo fork ---

cmd_repo_fork_help() {
    cat <<'EOF'
ghx repo fork — fork a repository

Usage: ghx repo fork [REPO] [options]

Options:
  --clone          Clone the fork locally
  --remote-name N  Name for the new remote (default: origin)
  --org ORG        Fork into an organization
  --learn          Learn about forking repos
EOF
}

cmd_repo_fork() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --clone)       args+=(--clone); shift ;;
            --remote-name) args+=(--remote-name "$2"); shift 2 ;;
            --org)         args+=(--org "$2"); shift 2 ;;
            *)             args+=("$1"); shift ;;
        esac
    done
    gh repo fork "${args[@]}"
}
