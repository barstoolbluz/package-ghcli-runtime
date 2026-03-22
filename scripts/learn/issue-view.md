# Viewing an Issue

## What This Does

Displays the full details of a specific issue — title, description, labels,
assignees, and optionally the comment thread.

## The Underlying Command

```
gh issue view <NUMBER> [--web --comments --json fields]
```

## When You'd Use This

- Reading the full context of an issue before working on it
- Checking the discussion and status without leaving the terminal
- Opening an issue in the browser for editing or commenting
- Extracting issue data for automation

## Options Explained

**`ISSUE_NUMBER`** — The issue number (e.g., `15`). Required unless piped.

**`-w, --web`** — Open in your browser instead of terminal. Use when you
need to edit, comment, or manage labels through the web UI.

**`--comments`** — Show the full comment thread. Without this, you only see
the original issue body.

**`--json`** — JSON output with specific fields. Useful fields: `title`,
`body`, `state`, `labels`, `assignees`, `comments`, `milestone`.

## Common Patterns

```bash
# Read an issue
ghx issue view 15

# Open in browser to comment
ghx issue view 15 --web

# Read the full discussion
ghx issue view 15 --comments

# Get assignee info
ghx issue view 15 --json assignees,labels
```

## Gotchas & Tips

- The terminal view is a great quick read, but for long issues with lots of
  markdown, `--web` gives a better formatted view.
- Use `--json` with `jq` for scripting:
  `ghx issue view 15 --json labels | jq '.labels[].name'`
- If an issue references PRs, those links show up in the terminal view too.
