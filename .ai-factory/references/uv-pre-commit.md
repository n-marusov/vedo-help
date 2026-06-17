# uv pre-commit Hooks Reference

> Source:
> - https://docs.astral.sh/uv/guides/integration/pre-commit/
> - https://github.com/astral-sh/uv-pre-commit
> - https://raw.githubusercontent.com/astral-sh/uv-pre-commit/main/.pre-commit-hooks.yaml
> Created: 2026-06-17
> Updated: 2026-06-17

## Overview

Astral provides official pre-commit hooks for uv in the `astral-sh/uv-pre-commit` repository. These hooks allow you to automate uv workflows — lockfile management, dependency export, requirements compilation, dependency synchronization, and vulnerability auditing — directly in your pre-commit pipeline.

The hooks are distributed as a standalone repository so uv can be installed via prebuilt wheels from PyPI. Pin the `rev` to a specific uv version.

## Available Hooks

| Hook ID | Purpose | Trigger Files | Stages |
|---------|---------|---------------|--------|
| `uv-lock` | Keep `uv.lock` in sync with `pyproject.toml` / `uv.toml` | `uv.lock`, `pyproject.toml`, `uv.toml` | `pre-commit` (default) |
| `uv-export` | Export `uv.lock` to `requirements.txt` | `uv.lock` | `pre-commit` (default) |
| `pip-compile` | Compile `.in` → `.txt` requirements files | `requirements.in`, `requirements.txt` | `pre-commit` (default) |
| `uv-sync` | Sync deps on checkout/pull/rebase | Always runs | `post-checkout`, `post-merge`, `post-rewrite` |
| `uv-audit` | Audit dependencies for vulnerabilities | `uv.lock`, `pyproject.toml`, `uv.toml` | `pre-commit` (default) |

## Hook Definitions

Full `.pre-commit-hooks.yaml` signatures:

```yaml
- id: pip-compile
  name: pip-compile
  description: "Automatically run 'uv pip compile' on your requirements"
  entry: uv pip compile
  language: python
  files: ^requirements\.(in|txt)$
  args: []
  pass_filenames: false
  additional_dependencies: []
  minimum_pre_commit_version: "2.9.2"

- id: uv-lock
  name: uv-lock
  description: "Automatically run 'uv lock' on your project dependencies"
  entry: uv lock
  language: python
  files: ^(uv\.lock|pyproject\.toml|uv\.toml)$
  args: []
  pass_filenames: false
  additional_dependencies: []
  minimum_pre_commit_version: "2.9.2"

- id: uv-export
  name: uv-export
  description: "Automatically run 'uv export' on your project dependencies"
  entry: uv export
  language: python
  files: ^uv\.lock$
  args: ["--frozen", "--output-file=requirements.txt", "--quiet"]
  pass_filenames: false
  additional_dependencies: []
  minimum_pre_commit_version: "2.9.2"

- id: uv-sync
  name: uv-sync
  description: "Automatically run 'uv sync' on your repository after a checkout, pull or rebase"
  entry: uv sync --no-active
  args: ["--locked"]
  language: python
  always_run: true
  pass_filenames: false
  stages: [post-checkout, post-merge, post-rewrite]
  minimum_pre_commit_version: "2.9.2"

- id: uv-audit
  name: uv-audit
  description: "Automatically run 'uv audit' on your project dependencies"
  entry: uv audit
  language: python
  files: ^(uv\.lock|pyproject\.toml|uv\.toml)$
  args: []
  pass_filenames: false
  additional_dependencies: []
  minimum_pre_commit_version: "2.9.2"
```

## Usage Patterns

### Lockfile sync

```yaml
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: 0.11.21
    hooks:
      - id: uv-lock
```

Triggers when `pyproject.toml`, `uv.toml`, or `uv.lock` changes. Runs `uv lock` to update `uv.lock`.

### Export lockfile to requirements.txt

```yaml
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: 0.11.21
    hooks:
      - id: uv-export
```

Default args: `["--frozen", "--output-file=requirements.txt", "--quiet"]`. Override via `args` for custom output:

```yaml
      - id: uv-export
        args: ["--frozen", "--output-file=requirements-custom.txt", "--quiet"]
```

### Compile requirements files

```yaml
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: 0.11.21
    hooks:
      - id: pip-compile
        args: [requirements.in, -o, requirements.txt]
```

For alternative files:

```yaml
      - id: pip-compile
        args: [requirements-dev.in, -o, requirements-dev.txt]
        files: ^requirements-dev\.(in|txt)$
```

Multiple files in one config:

```yaml
      - id: pip-compile
        name: pip-compile requirements.in
        args: [requirements.in, -o, requirements.txt]
      - id: pip-compile
        name: pip-compile requirements-dev.in
        args: [requirements-dev.in, -o, requirements-dev.txt]
        files: ^requirements-dev\.(in|txt)$
```

### Sync dependencies after checkout/pull/merge

```yaml
default_install_hook_types:
  - pre-commit
  - post-checkout
  - post-merge
  - post-rewrite
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: 0.11.21
    hooks:
      - id: uv-sync
```

Also install manually: `pre-commit install --install-hooks -t post-checkout -t post-merge -t post-rewrite`.

For a workspace with all packages:

```yaml
      - id: uv-sync
        args: ["--locked", "--all-packages"]
```

If using `keyring` for a private index:

```yaml
      - id: uv-sync
        additional_dependencies: [keyring]
```

### Audit dependencies

```yaml
repos:
  - repo: https://github.com/astral-sh/uv-pre-commit
    rev: 0.11.21
    hooks:
      - id: uv-audit
```

### Monorepo subdirectory

Run a hook on a project in a subdirectory:

```yaml
      - id: uv-lock
        files: <path/to/project>/pyproject.toml
        args: [--project, <path/to/project>]
```

## Configuration

All hooks share the same config surface:

| Field | Description |
|-------|-------------|
| `repo` | `https://github.com/astral-sh/uv-pre-commit` |
| `rev` | uv version tag (e.g. `0.11.21`). Use latest stable. |
| `hooks[].id` | One of: `uv-lock`, `uv-export`, `pip-compile`, `uv-sync`, `uv-audit` |
| `hooks[].args` | Override default CLI arguments |
| `hooks[].files` | Override file pattern that triggers the hook |
| `hooks[].name` | Override display name (useful for multiple pip-compile entries) |
| `hooks[].additional_dependencies` | Extra pip dependencies (e.g. `[keyring]`) |
| `default_install_hook_types` | Required for `uv-sync` post-checkout/post-merge/post-rewrite stages |

## Best Practices

1. **Pin a specific `rev`** — use an exact uv version tag, not a branch name, to ensure reproducible behavior.
2. **Use `uv-lock` as a safety net** — it guarantees `uv.lock` is never stale when `pyproject.toml` changes.
3. **Prefer `uv-export` over manual requirements.txt** — lets uv manage the export automatically with `--frozen` to avoid accidental lockfile changes.
4. **Combine `uv-lock` + `uv-export`** — run `uv-lock` first (updates lockfile), then `uv-export` (regenerates requirements.txt from it).
5. **Add `uv-audit` for CI/CD** — catches known vulnerabilities in dependencies before they reach production.
6. **Use `uv-sync` with `default_install_hook_types`** — ensures deps are synced on branch switches without manual `uv sync`.
7. **Name multiple `pip-compile` hooks** — give each a unique `name` so pre-commit output is readable.
8. **Set `pass_filenames: false`** is already the default — don't override it, as uv doesn't use pre-commit's file list.
9. **Keep `rev` consistent** across all uv hooks entries in the same repo config to avoid uv version mismatches.

## Common Pitfalls

- **`uv-sync` not running on checkout** — `pre-commit install` does not install `post-checkout`/`post-merge`/`post-rewrite` hooks by default. Must either set `default_install_hook_types` or run manual install with `-t` flags.
- **`uv-export` overwrites custom requirements.txt** — if you have a manually maintained `requirements.txt`, `uv-export` will clobber it. Use `args` to write to a different output file, or remove the hook.
- **Lockfile conflicts** — `uv-lock` only checks files listed in `files:` pattern. If your lockfile is in a non-standard location, override `files` or use `args: [--project, <path>]`.
- **Slow pre-commit on large projects** — `uv-lock` can be slow on large dependency trees. It only runs when trigger files change, so it won't block every commit.
- **`pip-compile` with `pass_filenames: true`** — the hook declares `pass_filenames: false` explicitly; if overridden to `true`, uv may receive unexpected arguments.
- **`uv-audit` fails on new advisories** — treat it as a CI gate, not a development blocker. Consider running it in CI rather than pre-commit if it's too noisy.

## Version Notes

- Minimum pre-commit version: `2.9.2` for all hooks.
- `uv-audit` was added in a later uv release (available in 0.4.x+). Check the release notes for the exact version.
- The `rev` tag in `astral-sh/uv-pre-commit` mirrors uv releases. Always use the latest stable uv version.
- The only language used is `python` — the hook installs uv from PyPI via pre-commit's Python environment.
