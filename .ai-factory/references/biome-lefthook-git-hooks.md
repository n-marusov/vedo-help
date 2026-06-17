# Biome + Lefthook Git Hooks Reference

> Source: https://biomejs.dev/recipes/git-hooks/
> Created: 2026-06-17
> Updated: 2026-06-17

## Overview

Biome can be integrated with Git hooks to automatically format, lint, and apply safe fixes to staged files before committing or pushing. This reference covers Lefthook configuration — a fast, cross-platform, dependency-free hook manager installable via npm.

## Core Concepts

- **Lefthook**: hook manager installed via npm, configured in `lefthook.yml` at repo root
- **`staged_files`**: Lefthook variable holding the list of staged files
- **`push_files`**: Lefthook variable holding the list of files being pushed
- **`stage_fixed: true`**: re-stages files after Biome modifies them
- **`--staged`**: Biome CLI option to process only staged files (used in shell scripts, not needed with Lefthook's `{staged_files}`)
- **`--no-errors-on-unmatched`**: silences errors when no files match
- **`--files-ignore-unknown=true`**: lets Biome skip unsupported file types
- **`--colors=off`**: disables colored output for cleaner logs

## Installation

```bash
npm install -D @biomejs/biome lefthook
```

Or install Lefthook globally:

```bash
npm install -g lefthook
```

## Configuration (`lefthook.yml`)

Create `lefthook.yml` at the root of the Git repository. After configuring, run:

```bash
lefthook install
```

### Pre-commit: Check formatting and lint only

```yaml
pre-commit:
  commands:
    check:
      glob: "*.{js,ts,cjs,mjs,d.cts,d.mts,jsx,tsx,json,jsonc,css}"
      run: npx @biomejs/biome check --no-errors-on-unmatched --files-ignore-unknown=true --colors=off {staged_files}
```

### Pre-commit: Format, lint, and apply safe fixes (re-stages changes)

```yaml
pre-commit:
  commands:
    check:
      glob: "*.{js,ts,cjs,mjs,d.cts,d.mts,jsx,tsx,json,jsonc,css}"
      run: npx @biomejs/biome check --write --no-errors-on-unmatched --files-ignore-unknown=true --colors=off {staged_files}
      stage_fixed: true
```

`stage_fixed: true` re-adds the modified files to the staging area.

### Pre-push: Check formatting and lint

```yaml
pre-push:
  commands:
    check:
      glob: "*.{js,ts,cjs,mjs,d.cts,d.mts,jsx,tsx,json,jsonc,css}"
      run: npx @biomejs/biome check --no-errors-on-unmatched --files-ignore-unknown=true --colors=off {push_files}
```

## Usage Patterns

### Pattern 1: Pre-commit with safe auto-fix + staging

This is the most common setup — Biome formats and lints staged files, applies safe fixes, and re-stages them so the commit includes the fixes.

```yaml
pre-commit:
  commands:
    check:
      glob: "*.{js,ts,cjs,mjs,d.cts,d.mts,jsx,tsx,json,jsonc,css}"
      run: npx @biomejs/biome check --write --no-errors-on-unmatched --files-ignore-unknown=true --colors=off {staged_files}
      stage_fixed: true
```

### Pattern 2: Separate lint and format steps (two commands)

Lint first, then format:

```yaml
pre-commit:
  commands:
    lint:
      glob: "*.{js,ts,jsx,tsx}"
      run: npx @biomejs/biome lint --write --no-errors-on-unmatched --colors=off {staged_files}
      stage_fixed: true
    format:
      glob: "*.{js,ts,jsx,tsx,json,jsonc,css}"
      run: npx @biomejs/biome format --write --no-errors-on-unmatched --colors=off {staged_files}
      stage_fixed: true
```

### Pattern 3: Without `glob` — let Biome auto-detect supported files

```yaml
pre-commit:
  commands:
    check:
      run: npx @biomejs/biome check --no-errors-on-unmatched --files-ignore-unknown=true --colors=off {staged_files}
      stage_fixed: true
```

Note from Biome docs: you don't need both `glob` and `--files-ignore-unknown=true`. Using only `--files-ignore-unknown=true` handles files supported now and in the future. Use `glob` when you want more control.

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `glob` | (none) | File pattern to match against `{staged_files}` |
| `run` | (required) | Command to execute with `{staged_files}` or `{push_files}` |
| `stage_fixed` | `false` | If `true`, re-stages files modified by the command |

Lefthook variables:

| Variable | Context | Description |
|----------|---------|-------------|
| `{staged_files}` | pre-commit | Files currently staged |
| `{push_files}` | pre-push | Files being pushed |

## Best Practices

1. **Use `--no-errors-on-unmatched`** — prevents hook failure when no staged files match the glob
2. **Use `--colors=off`** — keeps hook output clean in terminal logs
3. **Re-staging with `stage_fixed: true`** — ensures auto-fixed files are included in the commit
4. **Run `lefthook install` after cloning** — activates the hooks (must be run once per clone)
5. **Pin Biome version in `package.json`** — avoids unexpected formatting changes from Biome updates
6. **Keep `lefthook.yml` in version control** — all team members get the same hooks

## Common Pitfalls

- **Missing `lefthook install`**: hooks won't run until installed
- **No `stage_fixed: true` with `--write`**: auto-fixes are applied to the working tree but not committed
- **`{staged_files}` vs `{push_files}` mix-up**: `{staged_files}` is only available in pre-commit; `{push_files}` only in pre-push
- **Unstaged changes conflict with `--write`**: if a file has both staged and unstaged changes, running `--write` can produce confusing results; Biome's shell script example (see below) detects `MM` status and aborts in that case

## Shell Script Fallback

If you prefer not to use Lefthook, you can use a raw shell script as a pre-commit hook:

```sh
#!/bin/sh
set -eu

# Abort if any staged file also has unstaged changes
if git status --short | grep --quiet '^MM'; then
  printf '%s\n' "ERROR: Some staged files have unstaged changes" >&2
  exit 1
fi

npx @biomejs/biome check --write --staged --files-ignore-unknown=true --no-errors-on-unmatched
git update-index --again
```

## Alternative Hook Managers

The Biome docs also cover:

- **Husky + lint-staged**: widely used in JS ecosystem; Husky + `.husky/pre-commit` file calling `lint-staged`, configured in `package.json`
- **Husky + git-format-staged**: avoids `git stash` conflicts via stdin-based formatting
- **pre-commit**: multi-language hook manager; official hooks at `biomejs/pre-commit` (biome-ci, biome-check, biome-format, biome-lint)
