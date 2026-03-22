# Forking a Repository

## What This Does

Creates your own copy of someone else's repository under your GitHub account.
This is the standard way to contribute to open-source projects you don't have
write access to.

## The Underlying Command

```
gh repo fork [owner/repo] [--clone --org my-org --remote-name upstream]
```

## When You'd Use This

- You want to contribute to an open-source project
- You need your own copy to experiment with
- Your team uses a fork-based workflow

## Options Explained

**`REPO`** — The repo to fork. Defaults to the current directory's repo.

**`--clone`** — Also clone the fork to your machine. Saves a step since
you usually want to work on it immediately.

**`--remote-name`** — Name for the new remote pointing to your fork.
Defaults to `origin` (the original becomes `upstream`).

**`--org`** — Fork into an organization instead of your personal account.
Useful for company-internal forks.

## Common Patterns

```bash
# Fork and clone in one step
ghx repo fork cli/cli --clone

# Fork into your org
ghx repo fork cli/cli --org my-company

# Fork the current repo (you already cloned it)
ghx repo fork
```

## Gotchas & Tips

- **Fork workflow**: fork → clone → branch → commit → push to your fork →
  open PR against the original. `ghx repo fork --clone` handles the first two.
- If you've already cloned the original repo, running `ghx repo fork` (no args)
  will fork it and add your fork as a remote.
- GitHub forks are linked — PRs from your fork automatically target the
  original repo. This is different from just cloning.
- You only need to fork once per repo. If you already have a fork, `gh` will
  tell you and use the existing one.
- Keep your fork in sync: `git fetch upstream && git merge upstream/main`
