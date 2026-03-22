# Cloning a Repository

## What This Does

Downloads a full copy of a GitHub repository to your machine so you can work
on it locally. This is usually the first thing you do when starting work on
a project.

## The Underlying Command

```
gh repo clone <owner/repo> [directory]
```

This is like `git clone` but smarter — it automatically configures the remote
URL using your GitHub auth, so SSH vs HTTPS "just works."

## When You'd Use This

- Starting work on a new project
- Getting a fresh copy of a repo you contribute to
- Cloning someone else's project to explore or contribute

## Options Explained

**`REPO`** — The repository to clone. Accepts:
- `owner/name` format: `cli/cli`
- Full URL: `https://github.com/cli/cli`
- Just `name` if you're the owner: `my-project`

**`DIRECTORY`** — Where to put it. Defaults to the repo name.

## Common Patterns

```bash
# Clone by owner/name
ghx repo clone cli/cli

# Clone into a specific directory
ghx repo clone cli/cli ~/projects/gh-source

# Clone your own repo (just the name)
ghx repo clone my-project
```

## Gotchas & Tips

- `gh repo clone` uses your configured git protocol (SSH or HTTPS) automatically
  based on your `gh` auth setup. No need to choose manually.
- After cloning, `cd` into the directory to start working. `gh` and `ghx`
  commands need to be run from inside a repo directory.
- For repos you want to contribute to (but don't own), consider `ghx repo fork`
  first, which clones your fork and sets up the upstream remote.
- Large repos? Git supports shallow clones (`git clone --depth 1`) but `gh`
  doesn't expose this flag directly. Use `git clone` for advanced clone options.
