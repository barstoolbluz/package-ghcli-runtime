#!/usr/bin/env bash
# ghx run — CI/workflow run monitoring

cmd_run_help() {
    cat <<'EOF'
ghx run — CI/workflow monitoring

Subcommands:
  list       List recent workflow runs
  view       View a workflow run
  watch      Watch a run in progress

Use 'ghx run <subcommand> --help' for details.
Add '--learn' to any subcommand for an educational walkthrough.
EOF
}

# --- run list ---

cmd_run_list_help() {
    cat <<'EOF'
ghx run list — list recent workflow runs

Usage: ghx run list [options]

Options:
  -w, --workflow NAME  Filter by workflow name or file
  -b, --branch BRANCH  Filter by branch
  -s, --status STATUS  Filter: completed, in_progress, queued, etc.
  -L, --limit N        Max results (default: 20)
  --json FIELDS        Output as JSON
  --learn              Learn about listing runs
EOF
}

cmd_run_list() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--workflow) need_arg "$@"; args+=(--workflow "$2"); shift 2 ;;
            -b|--branch)  need_arg "$@"; args+=(--branch "$2"); shift 2 ;;
            -s|--status)  need_arg "$@"; args+=(--status "$2"); shift 2 ;;
            -L|--limit)   need_arg "$@"; args+=(--limit "$2"); shift 2 ;;
            --json)       need_arg "$@"; args+=(--json "$2"); shift 2 ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh run list "${args[@]}"
}

# --- run view ---

cmd_run_view_help() {
    cat <<'EOF'
ghx run view — view a workflow run

Usage: ghx run view [RUN_ID] [options]

Options:
  -w, --web         Open in browser
  --json FIELDS     Output as JSON
  --log             Show full log output
  --log-failed      Show only failed step logs
  --learn           Learn about viewing runs
EOF
}

cmd_run_view() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--web)       args+=(--web); shift ;;
            --json)         need_arg "$@"; args+=(--json "$2"); shift 2 ;;
            --log)          args+=(--log); shift ;;
            --log-failed)   args+=(--log-failed); shift ;;
            *)              args+=("$1"); shift ;;
        esac
    done
    gh run view "${args[@]}"
}

# --- run watch ---

cmd_run_watch_help() {
    cat <<'EOF'
ghx run watch — watch a run in progress

Usage: ghx run watch [RUN_ID] [options]

Options:
  -i, --interval SEC   Refresh interval (default: 3)
  --exit-status        Exit with run's conclusion status code
  --learn              Learn about watching runs
EOF
}

cmd_run_watch() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -i|--interval)   need_arg "$@"; args+=(--interval "$2"); shift 2 ;;
            --exit-status)   args+=(--exit-status); shift ;;
            *)               args+=("$1"); shift ;;
        esac
    done
    gh run watch "${args[@]}"
}
