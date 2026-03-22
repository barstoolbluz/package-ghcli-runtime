# GitHub Status Dashboard

## What This Does

Shows a combined overview of your GitHub activity for the current repo:
your open pull requests, issues assigned to you, and recent failing CI runs.
One command to answer "what needs my attention?"

## The Underlying Commands

Under the hood, `ghx status` runs three `gh` commands:
```
gh pr list --author @me --limit 10
gh issue list --assignee @me --limit 10
gh run list --status failure --limit 5
```

## When You'd Use This

- Starting your workday — "what's on my plate?"
- Before a standup — quick summary of your open work
- After a break — catching up on what might need attention
- Quick health check on CI

## How to Read the Output

The dashboard has three sections:

**Pull Requests** — Your open PRs. Check if any need rebasing, have
review comments, or are ready to merge.

**Assigned Issues** — Issues assigned to you. These are your current tasks.

**Recent Failed Runs** — CI failures in the repo. Even if they're not yours,
failing CI affects the whole team.

## Common Patterns

```bash
# Quick morning check
ghx status

# Use --learn to see this guide
ghx status --learn
```

## Gotchas & Tips

- This only shows data for the current repo. `cd` into different repos
  to check each one.
- If you're not in a repo directory, you'll get errors (but they're caught
  gracefully).
- For a cross-repo view of all your GitHub notifications, check out
  `gh status` (the built-in GitHub CLI command) which shows mentions,
  review requests, and more across all repos.
- The failed runs section shows repo-wide failures, not just yours. This
  is intentional — broken CI is everyone's problem.
