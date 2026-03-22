# Viewing a Repository

## What This Does

Shows information about a GitHub repository — description, stars, language,
license, and the README content. A quick way to learn about a project.

## The Underlying Command

```
gh repo view [owner/repo] [--web --json fields]
```

If you omit the repo name, it shows info for the current directory's repo.

## When You'd Use This

- Getting a quick overview of an unfamiliar repo
- Checking a repo's stats (stars, forks, license)
- Reading the README without opening a browser
- Extracting repo metadata for scripts

## Options Explained

**`REPO`** — Optional. `owner/name` format. Defaults to current directory's repo.

**`-w, --web`** — Open the repo page in your browser.

**`--json`** — JSON output. Fields: `name`, `description`, `url`, `stargazerCount`,
`forkCount`, `primaryLanguage`, `licenseInfo`, `isPrivate`, `defaultBranchRef`.

## Common Patterns

```bash
# View current repo
ghx repo view

# View a specific repo
ghx repo view golang/go

# Open in browser
ghx repo view golang/go --web

# Get metadata
ghx repo view golang/go --json stargazerCount,forkCount,primaryLanguage
```

## Gotchas & Tips

- The terminal view includes the full README, which can be long. Pipe to
  `less` or `head` if needed: `ghx repo view | head -50`
- `--json` is excellent for building dashboards or comparing repos.
- Private repos work too, as long as your `gh` token has access.
