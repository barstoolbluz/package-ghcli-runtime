# Watching a Workflow Run

## What This Does

Provides a live, updating view of a running workflow. Think of it as `tail -f`
for your CI pipeline — it refreshes automatically until the run finishes.

## The Underlying Command

```
gh run watch <RUN_ID> [--interval 3 --exit-status]
```

## When You'd Use This

- You just pushed and want to watch CI in real-time
- Waiting for a deployment pipeline to complete
- You want your terminal to exit with the run's status code (for scripting)

## Options Explained

**`RUN_ID`** — The run to watch. Get this from `ghx run list`. If you just
pushed, the most recent run is usually what you want.

**`-i, --interval`** — How often to refresh, in seconds. Default is 3.
Increase this for long-running pipelines to reduce API calls.

**`--exit-status`** — Exit with a non-zero status code if the run fails.
Essential for scripts that need to know if CI passed.

## Common Patterns

```bash
# Watch the latest run
ghx run watch 12345

# Watch with less frequent updates
ghx run watch 12345 -i 10

# In a script: fail if CI fails
ghx run watch 12345 --exit-status || echo "CI failed!"

# Push and watch (combine with gh)
git push && ghx run list -L 1 --json databaseId | jq '.[0].databaseId'
# then: ghx run watch <that-id>
```

## Gotchas & Tips

- `watch` only works on in-progress or queued runs. For completed runs,
  use `ghx run view` instead.
- The display auto-clears and updates. Press Ctrl+C to stop watching
  (the run itself keeps going — you're just disconnecting your view).
- `--exit-status` is great for local scripts: push, watch, and get a
  clear pass/fail signal without checking the web.
- API rate limits apply. With the default 3-second interval, you'll use
  about 20 API calls per minute. Fine for personal use, but bump the
  interval up for shared tokens.
