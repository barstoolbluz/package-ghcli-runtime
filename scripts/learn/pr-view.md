# Viewing a Pull Request

## What This Does

Shows the details of a specific pull request — title, description, status,
review state, and CI checks. Think of it as reading the PR page without
opening your browser.

## The Underlying Command

```
gh pr view [NUMBER] [--web --comments --json fields]
```

If you omit the PR number, `gh` shows the PR for your current branch.

## When You'd Use This

- Reading a PR's description and discussion from the terminal
- Checking the review and CI status before merging
- Quickly opening a PR in your browser with `--web`
- Extracting PR data for scripts with `--json`

## Options Explained

**`PR_NUMBER`** — The PR number (e.g., `42`). If omitted, uses the PR
associated with your current git branch.

**`-w, --web`** — Opens the PR in your default browser instead of showing
it in the terminal. Quick way to jump to the web UI.

**`--comments`** — Include the comment thread in the output. Without this,
you only see the PR description.

**`--json`** — Output specific fields as JSON. Useful fields: `title`, `body`,
`state`, `reviews`, `statusCheckRollup`, `mergeable`.

## Common Patterns

```bash
# View PR for current branch
ghx pr view

# View a specific PR
ghx pr view 42

# Open in browser
ghx pr view 42 --web

# Check CI status in JSON
ghx pr view 42 --json statusCheckRollup

# Read the full discussion
ghx pr view 42 --comments
```

## Gotchas & Tips

- Without a PR number, `gh` looks for a PR from your current branch. If there
  isn't one, it'll error — not fall back to listing PRs.
- The `--json` output is great for scripting. Combine with `jq` for filtering:
  `ghx pr view 42 --json reviews | jq '.reviews[] | .state'`
- Use `--web` when you need to do things the CLI can't, like resolving
  conversations or viewing file-level comments inline.
