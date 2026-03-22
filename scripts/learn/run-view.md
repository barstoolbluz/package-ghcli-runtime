# Viewing a Workflow Run

## What This Does

Shows details about a specific GitHub Actions workflow run — which jobs ran,
their status, duration, and optionally the full logs.

## The Underlying Command

```
gh run view <RUN_ID> [--web --log --log-failed --json fields]
```

## When You'd Use This

- A CI run failed and you want to see why
- You want to check the logs without going to the browser
- You need to see which specific job or step failed
- Extracting run data for reporting

## Options Explained

**`RUN_ID`** — The numeric run ID. Get this from `ghx run list`.

**`-w, --web`** — Open the run in your browser. Best for complex multi-job
workflows where you want the visual layout.

**`--log`** — Dump the full log output. Can be very long — pipe to `less`
or `grep` for specific errors.

**`--log-failed`** — Show only logs from failed steps. This is usually what
you want when debugging — skip the noise, see the errors.

**`--json`** — JSON output. Fields: `name`, `status`, `conclusion`, `jobs`,
`url`, `createdAt`, `updatedAt`.

## Common Patterns

```bash
# View run summary
ghx run view 12345

# See why it failed (just failed steps)
ghx run view 12345 --log-failed

# Full logs piped to less
ghx run view 12345 --log | less

# Search logs for errors
ghx run view 12345 --log | grep -i error

# Open in browser for the visual view
ghx run view 12345 --web
```

## Gotchas & Tips

- `--log-failed` is almost always what you want for debugging. The full `--log`
  can be thousands of lines.
- If the run has multiple jobs, `gh run view` shows a summary first. You can
  then view a specific job's logs.
- Logs are only available for completed runs. In-progress runs show the current
  status instead — use `ghx run watch` for live monitoring.
- Old run logs expire (GitHub retains them for 90 days by default).
