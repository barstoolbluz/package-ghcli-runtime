# Creating a Pull Request

## What This Does

Opens a pull request from your current branch to a target branch (usually `main`).
A PR is a proposal to merge your changes — it's where code review, discussion,
and CI checks happen before code lands.

## The Underlying Command

```
gh pr create [--title "..." --body "..." --draft --base main]
```

Under the hood, `ghx pr create` calls `gh pr create` with the flags you pass.

## When You'd Use This

- You've committed work on a feature branch and want feedback
- You're ready (or nearly ready) to merge into the main branch
- You want CI to run against your changes before merging
- You want to open a draft PR early to show progress

## Options Explained

**`-t, --title`** — The PR title. Shows up in lists and notifications. Keep it
concise but descriptive. If you skip this, `gh` will prompt you interactively.

**`-b, --body`** — The PR description. Explain *why* you made the change, not
just *what* changed (reviewers can read the diff for that). Supports Markdown.

**`-d, --draft`** — Creates a draft PR. This signals "not ready for review yet"
but still lets CI run and gives people visibility. Great for early feedback.

**`-B, --base`** — The branch you're merging *into*. Defaults to the repo's
default branch (usually `main`). Use this when targeting a release branch.

**`-f, --fill`** — Auto-fills the title and body from your commit messages.
Handy when your commits already tell the story.

**`-a, --assignee`** — Assign a reviewer. Use GitHub usernames.

**`-l, --label`** — Tag the PR with labels like `bug`, `enhancement`, etc.

## Common Patterns

```bash
# Simple PR with title
ghx pr create -t "Add user avatar upload"

# Draft PR (work in progress)
ghx pr create -d -t "WIP: Refactor auth middleware"

# Fill from commits, target a specific branch
ghx pr create -f -B release/v2

# Full PR with description
ghx pr create -t "Fix timeout on large uploads" \
  -b "Increases the upload timeout from 30s to 120s. Fixes #234."
```

## Gotchas & Tips

- You must push your branch before creating a PR. If you haven't pushed yet,
  `gh` will offer to push for you.
- If you don't pass `--title`, you'll get an interactive editor. This is fine
  for one-offs but scripts should always pass `--title`.
- Draft PRs don't trigger review requests — use them freely for early visibility.
- The `--fill` flag uses your commit messages, so write good commits!
- For complex PRs, consider opening a draft first, then marking it ready after
  CI passes: `gh pr ready`
