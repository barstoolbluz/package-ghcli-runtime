# Listing Workflow Runs

## What This Does

Shows recent GitHub Actions workflow runs for the current repository. This is
your CI/CD dashboard — see what's passing, failing, or still running.

## The Underlying Command

```
gh run list [--workflow ci.yml --branch main --status failure --limit 20]
```

## When You'd Use This

- Checking if CI passed after pushing
- Finding which workflows are failing
- Monitoring deployment pipelines
- Reviewing recent CI activity on a branch

## Options Explained

**`-w, --workflow`** — Filter by workflow name or filename. Use the workflow
name (e.g., "CI") or filename (e.g., `ci.yml`).

**`-b, --branch`** — Filter by branch. Useful to see runs for just your branch.

**`-s, --status`** — Filter by status: `completed`, `in_progress`, `queued`,
`failure`, `success`, `cancelled`.

**`-L, --limit`** — How many to show. Default 20.

**`--json`** — JSON output. Fields: `databaseId`, `name`, `status`,
`conclusion`, `headBranch`, `url`, `createdAt`.

## Common Patterns

```bash
# Recent runs
ghx run list

# Failed runs only
ghx run list -s failure

# Runs on your branch
ghx run list -b my-feature-branch

# Specific workflow
ghx run list -w "CI"

# Last 50 runs as JSON
ghx run list -L 50 --json databaseId,conclusion,headBranch
```

## Gotchas & Tips

- Run IDs (the numbers in the leftmost column) are what you pass to
  `ghx run view` and `ghx run watch`.
- Status `completed` includes both successes and failures. Use `--status failure`
  or `--status success` to be specific.
- If your repo has many workflows, always use `-w` to filter. Otherwise the
  list mixes different workflows together.
- For real-time monitoring of a specific run, use `ghx run watch` instead.
