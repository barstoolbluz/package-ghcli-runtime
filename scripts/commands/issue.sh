#!/usr/bin/env bash
# ghx issue — issue management commands

cmd_issue_help() {
    cat <<'EOF'
ghx issue — issue management

Subcommands:
  create     Create a new issue
  list       List issues
  view       View an issue

Use 'ghx issue <subcommand> --help' for details.
Add '--learn' to any subcommand for an educational walkthrough.
EOF
}

# --- issue create ---

cmd_issue_create_help() {
    cat <<'EOF'
ghx issue create — create a new issue

Usage: ghx issue create [options]

Options:
  -t, --title TITLE      Issue title
  -b, --body BODY        Issue body
  -a, --assignee USER    Assign to user (repeatable)
  -l, --label LABEL      Add label (repeatable)
  -m, --milestone NAME   Set milestone
  -p, --project NAME     Add to project
  --learn                Learn about creating issues
EOF
}

cmd_issue_create() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -t|--title)     need_arg "$@"; args+=(--title "$2"); shift 2 ;;
            -b|--body)      need_arg "$@"; args+=(--body "$2"); shift 2 ;;
            -a|--assignee)  need_arg "$@"; args+=(--assignee "$2"); shift 2 ;;
            -l|--label)     need_arg "$@"; args+=(--label "$2"); shift 2 ;;
            -m|--milestone) need_arg "$@"; args+=(--milestone "$2"); shift 2 ;;
            -p|--project)   need_arg "$@"; args+=(--project "$2"); shift 2 ;;
            *)              args+=("$1"); shift ;;
        esac
    done
    gh issue create "${args[@]}"
}

# --- issue list ---

cmd_issue_list_help() {
    cat <<'EOF'
ghx issue list — list issues

Usage: ghx issue list [options]

Options:
  -s, --state STATE    Filter: open, closed, all (default: open)
  -a, --assignee USER  Filter by assignee (@me for yourself)
  -l, --label LABEL    Filter by label (repeatable)
  -L, --limit N        Max results (default: 30)
  -S, --search QUERY   Search query
  --json FIELDS        Output as JSON
  --learn              Learn about listing issues
EOF
}

cmd_issue_list() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -s|--state)    need_arg "$@"; args+=(--state "$2"); shift 2 ;;
            -a|--assignee) need_arg "$@"; args+=(--assignee "$2"); shift 2 ;;
            -l|--label)    need_arg "$@"; args+=(--label "$2"); shift 2 ;;
            -L|--limit)    need_arg "$@"; args+=(--limit "$2"); shift 2 ;;
            -S|--search)   need_arg "$@"; args+=(--search "$2"); shift 2 ;;
            --json)        need_arg "$@"; args+=(--json "$2"); shift 2 ;;
            *)             args+=("$1"); shift ;;
        esac
    done
    gh issue list "${args[@]}"
}

# --- issue view ---

cmd_issue_view_help() {
    cat <<'EOF'
ghx issue view — view an issue

Usage: ghx issue view [ISSUE_NUMBER] [options]

Options:
  -w, --web         Open in browser
  --json FIELDS     Output as JSON
  --comments        Show comments
  --learn           Learn about viewing issues
EOF
}

cmd_issue_view() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--web)     args+=(--web); shift ;;
            --json)       need_arg "$@"; args+=(--json "$2"); shift 2 ;;
            --comments)   args+=(--comments); shift ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh issue view "${args[@]}"
}
