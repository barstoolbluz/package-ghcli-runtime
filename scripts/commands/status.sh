#!/usr/bin/env bash
# ghx status — combined dashboard

cmd_status_help() {
    cat <<'EOF'
ghx status — your GitHub dashboard

Usage: ghx status [options]

Shows a combined view of:
  - Your open pull requests
  - Issues assigned to you
  - Recent failing workflow runs

Options:
  --learn    Learn about the status dashboard
EOF
}

cmd_status() {
    ensure_gh_auth

    fmt_header "Pull Requests"
    gh pr list --author "@me" --limit 10 2>/dev/null || fmt_info "No PRs found (or not in a repo)"
    printf '\n'

    fmt_header "Assigned Issues"
    gh issue list --assignee "@me" --limit 10 2>/dev/null || fmt_info "No assigned issues found (or not in a repo)"
    printf '\n'

    fmt_header "Recent Failed Runs"
    gh run list --status failure --limit 5 2>/dev/null || fmt_info "No failed runs found (or not in a repo)"
}
