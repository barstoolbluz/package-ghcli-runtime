# Listing Pull Requests

## What This Does

Shows pull requests for the current repository, filtered by state, author, labels,
or a search query. By default shows open PRs.

## The Underlying Command

```
gh pr list [--state open --author @me --label bug --limit 30]
```

## When You'd Use This

- Checking what PRs are open and need review
- Finding your own PRs across a repo
- Looking for PRs related to a specific label or feature
- Getting a quick overview before starting work

## Options Explained

**`-s, --state`** — Filter by PR state: `open` (default), `closed`, `merged`,
or `all`. Use `merged` to find past PRs that landed, `closed` for ones that
were abandoned.

**`-a, --author`** — Filter by who created the PR. Use `@me` for your own.
Use a GitHub username for someone else's.

**`-l, --label`** — Filter by label. Repeat the flag for multiple labels
(they're ANDed together — PR must have all listed labels).

**`-L, --limit`** — How many results to show. Default is 30. Set higher if
you're looking through history.

**`-S, --search`** — Free-text search using GitHub's search syntax. This is
powerful — you can use qualifiers like `review:required`, `draft:true`, etc.

**`--json`** — Output as JSON with specific fields. Useful for scripting.
Example fields: `number`, `title`, `state`, `author`, `url`.

## Common Patterns

```bash
# Your open PRs
ghx pr list -a @me

# All merged PRs this sprint
ghx pr list -s merged -L 50

# PRs labeled "needs-review"
ghx pr list -l needs-review

# Search for draft PRs
ghx pr list -S "draft:true"

# JSON output for scripting
ghx pr list --json number,title,url
```

## Gotchas & Tips

- `ghx pr list` only works inside a git repo that has a GitHub remote. If you
  get an error, make sure you're in the right directory.
- The `--search` flag uses GitHub search syntax, which is different from the
  other flags. Check GitHub docs for the full query language.
- For a quick "what needs my attention" view, try `ghx status` instead —
  it combines PRs, issues, and CI status.
