# Creating an Issue

## What This Does

Opens a new issue in the current repository. Issues are how you track bugs,
feature requests, tasks, and discussions. They're the to-do list of a project.

## The Underlying Command

```
gh issue create [--title "..." --body "..." --label bug --assignee @me]
```

## When You'd Use This

- You found a bug and want to report it
- You have a feature idea to propose
- You need to create a task for yourself or a teammate
- You want to document a problem for later

## Options Explained

**`-t, --title`** — The issue title. Make it specific and actionable.
"Login broken" is okay; "Login fails with OAuth when session expires" is better.

**`-b, --body`** — The full description. Include steps to reproduce for bugs,
or motivation and details for features. Supports Markdown.

**`-a, --assignee`** — Who should work on this. Use `@me` for yourself, or
a GitHub username. Repeat the flag for multiple assignees.

**`-l, --label`** — Categorize with labels like `bug`, `enhancement`, `docs`.
Repeat for multiple labels. Labels must already exist in the repo.

**`-m, --milestone`** — Associate with a milestone (sprint, release, etc.).

**`-p, --project`** — Add to a GitHub Project board.

## Common Patterns

```bash
# Quick bug report
ghx issue create -t "Search returns 500 on empty query" -l bug

# Feature request with description
ghx issue create -t "Add dark mode support" -l enhancement \
  -b "Users have requested dark mode. Should respect OS preference by default."

# Assign to yourself
ghx issue create -t "Update API docs for v2 endpoints" -a @me -l docs

# Interactive mode (opens editor)
ghx issue create
```

## Gotchas & Tips

- If you skip `--title`, you'll get an interactive editor. Nice for detailed
  issues, but scripts should always pass `--title`.
- Labels must already exist in the repo — you can't create new labels through
  the issue creation command.
- Use issue templates if the repo has them: `gh issue create --template bug_report`
- Reference other issues with `#123` syntax in the body — GitHub auto-links them.
- For quick bugs during development, a one-liner with `-t` and `-l bug` is
  often enough. You can always edit later on the web.
