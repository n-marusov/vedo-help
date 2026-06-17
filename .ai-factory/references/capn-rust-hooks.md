# capn — Rust Development Automation Hooks Reference

> Source:
> - https://github.com/bearcove/capn
> - https://raw.githubusercontent.com/bearcove/capn/main/CHANGELOG.md
> - https://raw.githubusercontent.com/bearcove/capn/main/hooks/pre-commit
> - https://raw.githubusercontent.com/bearcove/capn/main/hooks/pre-push
> - https://raw.githubusercontent.com/bearcove/capn/main/hooks/install.sh
> Created: 2026-06-17
> Updated: 2026-06-17

## Overview

**capn** is a development automation tool for Rust workspaces. It runs as pre-commit and pre-push hooks, handling code formatting and comprehensive validation. Forked from [facet-dev](https://github.com/facet-rs/facet-dev) and made generic for any Rust workspace.

It installs native git hooks (not via pre-commit framework) that call the `capn` binary. All checks run in parallel with live progress spinners; if any check fails, remaining tasks are cancelled immediately.

## Core Concepts

- **Pre-commit**: Runs on every commit. Formats staged Rust files with `rustfmt`, stages `Cargo.lock`, enforces edition 2024, checks for external path deps.
- **Pre-push**: Runs before pushing. Runs clippy, tests (`cargo nextest`), doc tests, doc builds, and `cargo-shear` for unused deps.
- **`HAVE_MERCY`**: Emergency escape hatch env var to skip slow checks (values 1–3 with increasing severity).
- **capn** vs **captain**: capn provides a `captain` compatibility shim that forwards to `capn`. Config migrated from `.config/captain/` to `.config/capn/config.styx`.
- **Parallel execution**: All pre-push checks run in parallel. First failure cancels remaining tasks.

## Installation

### Quick install (macOS/Linux)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bearcove/capn/releases/latest/download/capn-installer.sh | sh
```

### Quick install (Windows PowerShell)

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/bearcove/capn/releases/latest/download/capn-installer.ps1 | iex"
```

### From crates.io

```bash
cargo install capn
```

Installs both `capn` binary and a `captain` compatibility shim.

### From source

```bash
cargo install --git https://github.com/bearcove/capn
```

## Quick Start

```bash
# Initialize capn in your project
capn init

# Install git hooks
./hooks/install.sh
```

`capn init` creates:
- `hooks/pre-commit` — runs `capn` on commit
- `hooks/pre-push` — runs `capn pre-push` on push
- `hooks/install.sh` — copies hooks into `.git/hooks/` (supports worktrees)
- `.config/capn/config.styx` — configuration file

## Usage

### Run pre-commit checks

```bash
capn
# or equivalently:
cargo run --release --quiet --bin capn
```

### Run pre-push checks

```bash
capn pre-push
# or equivalently:
cargo run --release --quiet --bin capn -- pre-push
```

### Emergency skip

```bash
HAVE_MERCY=1 git push   # Skip tests, doc-tests, docs
HAVE_MERCY=2 git push   # Also skip clippy
HAVE_MERCY=3 git push   # Skip everything (formatting only)
```

### Debug workspace info

```bash
capn debug-packages
```

### Migrate from legacy config

```bash
capn migrate
```

Moves `.config/captain/` → `.config/capn/`. When both exist, `.config/capn/` takes precedence.

## Hook Scripts

The generated hooks are simple bash wrappers:

**`hooks/pre-commit`**:
```bash
#!/bin/bash
cargo run --release --quiet --bin capn
```

**`hooks/pre-push`**:
```bash
#!/bin/bash
cargo run --release --quiet --bin capn -- pre-push
```

**`hooks/install.sh`** copies these into `.git/hooks/` (including worktree hooks), ignores `install.sh` itself, and makes them executable.

## Configuration

File: `.config/capn/config.styx` (Styx format)

```styx
@schema {id crate:capn-config@1, cli capn}

pre-commit {
  generate-readmes false
  rustfmt true
  cargo-lock true
  arborium true
  edition-2024 true
  external-path-deps true
  internal-dev-deps-release-plz true
}

pre-push {
  clippy true
  nextest true
  doc-tests false
  docs true
  cargo-shear true
}
```

### Pre-commit Options

| Option | Default | Description |
|--------|---------|-------------|
| `generate-readmes` | `false` | Deprecated/ignored. If enabled, recommends `cargo-reedme` |
| `rustfmt` | `true` | Format staged Rust files with `rustfmt` (edition 2024) |
| `cargo-lock` | `true` | Stage `Cargo.lock` changes after dependency updates |
| `arborium` | `true` | Configure arborium syntax highlighting for rustdoc |
| `edition-2024` | `true` | Require all crates to use Rust edition 2024 |
| `external-path-deps` | `true` | Catch path dependencies pointing outside the workspace |
| `internal-dev-deps-release-plz` | `true` | Forbid internal dev-deps with `workspace = true` or `path` + `version` |

### Pre-push Options

| Option | Default | Description |
|--------|---------|-------------|
| `clippy` | `true` | Run `cargo clippy` with `-D warnings` |
| `nextest` | `true` | Run tests via `cargo nextest` (only affected crates) |
| `doc-tests` | `false` | Run documentation tests (disabled by default) |
| `docs` | `true` | Build docs with `cargo doc -D warnings` |
| `cargo-shear` | `true` | Check for unused dependencies with `cargo-shear` |
| `clippy-features` | — | Features for clippy (omit for `--all-features`) |
| `doc-test-features` | — | Features for doc tests |
| `docs-features` | — | Features for rustdoc |

## Best Practices

1. **Run `capn init` once per project** — it sets up hooks and config. Re-run if you need to regenerate hooks.
2. **Track `hooks/` in git** — commit the generated hooks so all contributors get the same setup via `./hooks/install.sh`.
3. **Use `HAVE_MERCY` sparingly** — it's an emergency escape hatch for urgent pushes, not a daily workflow.
4. **Keep `.config/capn/config.styx` in version control** — so all team members share the same check configuration.
5. **Enable `doc-tests` in CI only** — it's disabled by default in pre-push for speed; run in CI for coverage.
6. **Pair with nightly `rustfmt`** — capn uses edition 2024 formatting, which requires nightly `rustfmt`. Ensure CI uses the same.
7. **Run `capn` before commit** (not `git commit -n`) — skipping hooks defeats the purpose of automation.
8. **Migrate legacy configs** — if you have `.config/captain/`, run `capn migrate` to move to the new format.

## Common Pitfalls

- **`capn: command not found`** — capn installs to `~/.cargo/bin/`. Ensure it's in `PATH`, or install via the quick-install script which handles this.
- **Hooks not running after clone** — hooks are not tracked by git. Run `./hooks/install.sh` after cloning to install them into `.git/hooks/`.
- **Worktree hooks missing** — `install.sh` handles worktrees automatically by iterating over `.git/worktrees/*/hooks`. Re-run after creating new worktrees.
- **Slow pre-push on large workspaces** — all checks run in parallel, but if the workspace is huge, consider disabling `doc-tests` or `docs` locally and relying on CI.
- **`cargo nextest` not installed** — capn expects `cargo-nextest` for pre-push tests. Install it: `cargo install cargo-nextest`.
- **`cargo-shear` not installed** — required for unused deps check. Install: `cargo install cargo-shear`.
- **Edition 2024 enforcement fails on older Rust** — edition 2024 requires nightly Rust. Ensure your toolchain is configured accordingly.
- **`HAVE_MERCY=1` still slow** — level 1 only skips tests/docs. Level 2 also skips clippy; level 3 skips all validation.

## Version Notes

- capn v1.4.0 is the latest release (March 2026).
- Forked from facet-dev, renamed from "captain" to "capn".
- Config path changed from `.config/captain/` to `.config/capn/config.styx`.
- A `captain` compatibility shim is installed alongside `capn` for backward compatibility.
- `generate-readmes` is deprecated and ignored; `cargo-reedme` is the recommended replacement.
- The `@schema` line in config.styx requires the `capn-config` crate at version `@1`.
- Minimum Rust: edition 2024 formatting requires nightly Rust toolchain.
