# Checking Out a Pull Request

## What This Does

Fetches a PR's branch and switches to it locally so you can test the changes,
review the code, or continue working on it. This is how you go from "I see a
PR online" to "I have this code on my machine."

## The Underlying Command

```
gh pr checkout <NUMBER> [--branch local-name --detach]
```

## When You'd Use This

- A teammate asks you to review their PR and you want to run it locally
- You want to test a PR's changes before approving
- You're picking up someone else's PR to continue the work
- You want to try a contributor's fix on your own machine

## Options Explained

**`PR_NUMBER`** — Required. The PR number to check out.

**`-b, --branch`** — Override the local branch name. By default, `gh` uses the
PR's branch name. Use this if there's a naming conflict or you want something
shorter.

**`--detach`** — Check out in detached HEAD state (no local branch created).
Useful when you just want to look at the code without creating a branch.

## Common Patterns

```bash
# Check out PR #42
ghx pr checkout 42

# Check out with a custom branch name
ghx pr checkout 42 -b review-auth-fix

# Just look at the code (no branch)
ghx pr checkout 42 --detach
```

## Gotchas & Tips

- This fetches from the remote, so you need network access.
- If you have uncommitted changes, git may refuse to switch branches. Stash
  your work first: `git stash`, then check out, then `git stash pop` when done.
- For cross-fork PRs (someone forked and sent a PR), `gh pr checkout` handles
  the remote setup automatically — you don't need to manually add their fork.
- After checking out, you can push additional commits to the PR branch (if you
  have write access to the source repo/fork).
- When you're done reviewing, switch back: `git checkout main`
