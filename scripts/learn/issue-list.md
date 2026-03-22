# Listing Issues

## What This Does

Shows issues in the current repository, filtered by state, assignee, labels,
or search terms. Your starting point for "what needs doing."

## The Underlying Command

```
gh issue list [--state open --assignee @me --label bug --limit 30]
```

## When You'd Use This

- Checking your assigned work for the day
- Finding bugs that need attention
- Getting an overview of open issues before a sprint
- Searching for related issues before filing a new one

## Options Explained

**`-s, --state`** — `open` (default), `closed`, or `all`. Use `closed` to
find resolved issues, `all` when searching broadly.

**`-a, --assignee`** — Filter by who's responsible. `@me` is your best friend
here. Leave empty to see all issues.

**`-l, --label`** — Filter by label. Multiple `-l` flags are ANDed (must match
all labels).

**`-L, --limit`** — How many to show. Default 30. Increase for bigger backlogs.

**`-S, --search`** — GitHub search syntax. Powerful for complex queries.

**`--json`** — Machine-readable output. Fields: `number`, `title`, `state`,
`assignees`, `labels`, `url`, `createdAt`.

## Common Patterns

```bash
# What's assigned to me?
ghx issue list -a @me

# Open bugs
ghx issue list -l bug

# High priority items
ghx issue list -l "priority: high"

# Search for auth-related issues
ghx issue list -S "auth in:title,body"

# All issues (open + closed)
ghx issue list -s all -L 100

# JSON for scripting
ghx issue list --json number,title,labels
```

## Gotchas & Tips

- This shows issues for the current repo. Make sure you're in the right
  directory, or the results will be for the wrong project.
- Labels with spaces need quoting: `-l "good first issue"`
- The search flag uses GitHub's search syntax which supports qualifiers like
  `is:open`, `no:assignee`, `comments:>5`, etc.
- For a personal dashboard across all repos, try `ghx status` instead.
