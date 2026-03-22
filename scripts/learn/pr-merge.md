# Merging a Pull Request

## What This Does

Merges an approved pull request into its base branch. This is the final step
in the PR lifecycle — the code goes from "proposed" to "landed."

## The Underlying Command

```
gh pr merge [NUMBER] [--merge|--squash|--rebase] [--delete-branch] [--auto]
```

## When You'd Use This

- A PR is approved and CI is green — time to land it
- You want to squash a messy commit history into one clean commit
- You want to set up auto-merge so it lands as soon as CI passes

## Options Explained

**`-m, --merge`** — Create a merge commit. Preserves the full branch history.
This is the default on most repos.

**`-s, --squash`** — Squash all commits into one, then merge. Great when the
PR has lots of small "fix typo" commits you don't want in main.

**`-r, --rebase`** — Rebase the PR's commits onto the base branch. Gives a
linear history without merge commits.

**`-d, --delete-branch`** — Delete the source branch after merging. Keeps
your branch list clean. Highly recommended.

**`--auto`** — Enable auto-merge. The PR will merge automatically once all
required checks pass and reviews are approved. You can set this early and
walk away.

## Common Patterns

```bash
# Squash merge and clean up
ghx pr merge 42 --squash --delete-branch

# Merge commit (preserve history)
ghx pr merge 42 --merge -d

# Auto-merge when ready
ghx pr merge 42 --squash --auto -d

# Merge the PR for current branch
ghx pr merge --squash -d
```

## Gotchas & Tips

- If the repo requires reviews or passing checks, the merge will fail until
  those are satisfied. Use `--auto` to queue it up.
- **Squash vs merge vs rebase**: squash is best for feature PRs with messy
  history, merge for preserving context, rebase for a clean linear log.
  Most teams standardize on one strategy.
- `--delete-branch` only deletes the remote branch. Your local branch sticks
  around until you run `git branch -d branch-name`.
- If merge conflicts exist, you'll need to resolve them first (update your
  branch from main, fix conflicts, push).
- Auto-merge requires the repo to have this feature enabled in settings.
