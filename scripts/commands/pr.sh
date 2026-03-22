#!/usr/bin/env bash
# ghx pr — pull request workflow commands

cmd_pr_help() {
    cat <<'EOF'
ghx pr — pull request workflow

Subcommands:
  create     Create a new pull request
  list       List pull requests
  view       View a pull request
  checkout   Check out a pull request branch locally
  merge      Merge a pull request

Use 'ghx pr <subcommand> --help' for details.
Add '--learn' to any subcommand for an educational walkthrough.
EOF
}

# --- pr create ---

cmd_pr_create_help() {
    cat <<'EOF'
ghx pr create — create a new pull request

Usage: ghx pr create [options]

Options:
  -t, --title TITLE    PR title
  -b, --body BODY      PR body
  -d, --draft          Create as draft
  -B, --base BRANCH    Base branch (default: repo default)
  -a, --assignee USER  Assign to user
  -l, --label LABEL    Add label (repeatable)
  -f, --fill           Fill title/body from commits
  --learn              Learn about PR creation
EOF
}

cmd_pr_create() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -t|--title)   args+=(--title "$2"); shift 2 ;;
            -b|--body)    args+=(--body "$2"); shift 2 ;;
            -d|--draft)   args+=(--draft); shift ;;
            -B|--base)    args+=(--base "$2"); shift 2 ;;
            -a|--assignee) args+=(--assignee "$2"); shift 2 ;;
            -l|--label)   args+=(--label "$2"); shift 2 ;;
            -f|--fill)    args+=(--fill); shift ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh pr create "${args[@]}"
}

# --- pr list ---

cmd_pr_list_help() {
    cat <<'EOF'
ghx pr list — list pull requests

Usage: ghx pr list [options]

Options:
  -s, --state STATE    Filter by state: open, closed, merged, all (default: open)
  -a, --author USER    Filter by author
  -l, --label LABEL    Filter by label (repeatable)
  -L, --limit N        Max results (default: 30)
  -S, --search QUERY   Search query
  --json FIELDS        Output as JSON with specified fields
  --learn              Learn about listing PRs
EOF
}

cmd_pr_list() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -s|--state)   args+=(--state "$2"); shift 2 ;;
            -a|--author)  args+=(--author "$2"); shift 2 ;;
            -l|--label)   args+=(--label "$2"); shift 2 ;;
            -L|--limit)   args+=(--limit "$2"); shift 2 ;;
            -S|--search)  args+=(--search "$2"); shift 2 ;;
            --json)       args+=(--json "$2"); shift 2 ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh pr list "${args[@]}"
}

# --- pr view ---

cmd_pr_view_help() {
    cat <<'EOF'
ghx pr view — view a pull request

Usage: ghx pr view [PR_NUMBER] [options]

Options:
  -w, --web         Open in browser
  --json FIELDS     Output as JSON
  --comments        Show comments
  --learn           Learn about viewing PRs
EOF
}

cmd_pr_view() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--web)     args+=(--web); shift ;;
            --json)       args+=(--json "$2"); shift 2 ;;
            --comments)   args+=(--comments); shift ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh pr view "${args[@]}"
}

# --- pr checkout ---

cmd_pr_checkout_help() {
    cat <<'EOF'
ghx pr checkout — check out a PR branch locally

Usage: ghx pr checkout <PR_NUMBER> [options]

Options:
  -b, --branch NAME    Local branch name override
  --detach             Detach HEAD (no local branch)
  --learn              Learn about checking out PRs
EOF
}

cmd_pr_checkout() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -b|--branch)  args+=(--branch "$2"); shift 2 ;;
            --detach)     args+=(--detach); shift ;;
            *)            args+=("$1"); shift ;;
        esac
    done
    gh pr checkout "${args[@]}"
}

# --- pr merge ---

cmd_pr_merge_help() {
    cat <<'EOF'
ghx pr merge — merge a pull request

Usage: ghx pr merge [PR_NUMBER] [options]

Options:
  -m, --merge          Use merge commit
  -s, --squash         Squash and merge
  -r, --rebase         Rebase and merge
  -d, --delete-branch  Delete branch after merge
  --auto               Enable auto-merge
  --learn              Learn about merging PRs
EOF
}

cmd_pr_merge() {
    ensure_gh_auth
    local args=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -m|--merge)          args+=(--merge); shift ;;
            -s|--squash)         args+=(--squash); shift ;;
            -r|--rebase)         args+=(--rebase); shift ;;
            -d|--delete-branch)  args+=(--delete-branch); shift ;;
            --auto)              args+=(--auto); shift ;;
            *)                   args+=("$1"); shift ;;
        esac
    done
    gh pr merge "${args[@]}"
}
