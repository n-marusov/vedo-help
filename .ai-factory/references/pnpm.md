# pnpm Reference

> Source: pnpm CLI help (v11.9.0), https://github.com/pnpm/pnpm, local configuration
> Created: 2026-07-04
> Updated: 2026-07-04

## Overview

pnpm is a fast, disk-space-efficient package manager for Node.js. Version 11 introduces significant configuration changes: the `pnpm` field in `package.json` is no longer read, and build-script approval is now configured via `allowBuilds` in `.npmrc` or via `onlyBuiltDependencies` in `pnpm-workspace.yaml`.

## Installation & Setup

### Corepack (recommended)

```bash
# Enable corepack once
corepack enable

# Prepare specific version
corepack prepare pnpm@11.9.0 --activate

# Or let corepack auto-detect from package.json's packageManager field
corepack enable
# Then pnpm commands use the version from packageManager
```

### package.json

```json
{
  "packageManager": "pnpm@11.9.0"
}
```

### Docker

```dockerfile
FROM node:22-alpine
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate
COPY package.json pnpm-lock.yaml .npmrc ./
RUN pnpm install --frozen-lockfile
```

## Configuration

### v11 Breaking Change: `pnpm` field in `package.json`

In pnpm v11, the `pnpm` key in `package.json` is **no longer read**. The following warning confirms it:

```
[WARN] The "pnpm" field in package.json is no longer read by pnpm.
The following keys were ignored: "pnpm.onlyBuiltDependencies".
See https://pnpm.io/settings for the new home of each setting.
```

Configuration now lives in either:
1. **`.npmrc`** — simple key-value format
2. **`pnpm-workspace.yaml`** — YAML format (with `--location project`)

### Build Scripts: `allowBuilds` / `onlyBuiltDependencies`

In pnpm v11, packages that need to run build scripts (postinstall, prepare, etc.) must be explicitly approved. Otherwise `pnpm install` fails with:

```
[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: <pkgs>
Run "pnpm approve-builds" to pick which dependencies should be allowed to run scripts.
```

#### `.npmrc` format (one per line)

```ini
allowBuilds=@biomejs/biome
allowBuilds=esbuild
allowBuilds=lefthook
allowBuilds=protobufjs
allowBuilds=vue-demi
```

#### `pnpm-workspace.yaml` format (array)

```yaml
onlyBuiltDependencies:
  - "@biomejs/biome"
  - esbuild
  - lefthook
  - protobufjs
  - vue-demi
```

#### Interactive CLI

```bash
# Interactive approval prompt
pnpm approve-builds

# Approve specific packages non-interactively
pnpm approve-builds esbuild lefthook

# Deny specific packages
pnpm approve-builds !some-package

# Approve all pending without prompts
pnpm approve-builds --all
```

Running `pnpm approve-builds` creates a `.npmrc` file in the project directory with `allowBuilds=<pkg>` entries.

### Config CLI

```bash
# List all config
pnpm config list

# Get a specific key
pnpm config get <key>
pnpm config get --json <key>

# Set a key
pnpm config set <key> <value>

# Delete a key
pnpm config delete <key>

# Specify config location
# --location project: uses pnpm-workspace.yaml (if exists) or .npmrc
# --location global: global config file
pnpm config set --location project <key> <value>
```

### Config location behavior (`--location`)

| Value | File used | Created if missing |
|-------|-----------|-------------------|
| `project` | `pnpm-workspace.yaml` (preferred) or `.npmrc` | `pnpm-workspace.yaml` |
| `global` | Global config file | N/A |
| (default) | Merged from project + global | N/A |

## Common CLI Commands

### Dependency Management

| Command | Description |
|---------|-------------|
| `pnpm add <pkg>` | Install package as production dependency |
| `pnpm add -D <pkg>` | Install as dev dependency |
| `pnpm install` / `pnpm i` | Install all dependencies |
| `pnpm install --frozen-lockfile` | Install from lockfile (CI/Docker) |
| `pnpm install --prod` | Production-only install |
| `pnpm remove <pkg>` / `pnpm rm <pkg>` | Remove package |
| `pnpm update` / `pnpm up` | Update packages to latest within range |
| `pnpm update --latest` | Update to latest version (ignores range) |
| `pnpm link <path>` | Link local package |
| `pnpm unlink` | Unlink local package |

### Review

| Command | Description |
|---------|-------------|
| `pnpm audit` | Check for security vulnerabilities |
| `pnpm ls` / `pnpm list` | List installed packages |
| `pnpm outdated` | List outdated packages |
| `pnpm why <pkg>` | Show why a package is installed |

### Scripts

| Command | Description |
|---------|-------------|
| `pnpm run <script>` | Run a script from package.json |
| `pnpm dev` | Shorthand for "pnpm run dev" |
| `pnpm build` | Shorthand for "pnpm run build" |
| `pnpm exec <cmd>` | Execute command in project context |
| `pnpm dlx <pkg>` | Fetch and run package without installing as dependency |
| `pnpm create <starter>` | Create project from starter kit |

### Other

| Command | Description |
|---------|-------------|
| `pnpm init` | Create package.json |
| `pnpm publish` | Publish package to registry |
| `pnpm approve-builds` | Approve build scripts for dependencies |

## Package.json Scripts Compatibility

Scripts defined in `package.json` run identically with pnpm, npm, and yarn:

```json
"scripts": {
  "dev": "vite --host 0.0.0.0",
  "build": "vite build",
  "test": "vitest run"
}
```

```bash
pnpm dev    # same as: npm run dev
pnpm build  # same as: npm run build
pnpm test   # same as: npm test
```

## Docker Best Practices

### BuildKit caching

```dockerfile
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
```

### Multi-stage setup

```dockerfile
# Enable pnpm
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate

# Copy manifests (including .npmrc for build approvals)
COPY package.json pnpm-lock.yaml .npmrc ./

# Install with cache mount
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
```

The `.npmrc` file must be present in the image for `allowBuilds` entries to be read during `pnpm install`.

## CI (GitHub Actions)

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: "22"
    cache: "pnpm"
    cache-dependency-path: frontend/pnpm-lock.yaml

- name: Enable pnpm (corepack)
  run: corepack enable && corepack prepare pnpm@11.9.0 --activate

- name: Install dependencies
  run: pnpm install --frozen-lockfile
```

## Common Pitfalls

### `ERR_PNPM_IGNORED_BUILDS`

**Cause:** pnpm v11 blocks build scripts by default. Packages like `esbuild`, `@biomejs/biome`, `lefthook`, `protobufjs`, and `vue-demi` need explicit approval.

**Fix:** Run `pnpm approve-builds` locally, which creates `.npmrc` with `allowBuilds=<pkg>` entries. Then ensure `.npmrc` is present in all environments (Docker, CI).

### `"pnpm" field in package.json is no longer read`

**Cause:** pnpm v11 moved configuration out of `package.json`.

**Fix:** Move settings to `.npmrc` (for simple key-value) or `pnpm-workspace.yaml` (for structured YAML).

### Lockfile format mismatch

When upgrading from pnpm v10 to v11, run `pnpm install` to regenerate the lockfile. The lockfile format may differ (`lockfileVersion` in `pnpm-lock.yaml`).

### Stale pnpm store in Docker

When the BuildKit cache mount is stale, the install may fail with unexpected errors. Clear the cache with:

```bash
docker builder prune --filter type=exec.cachemount
```

## Version Notes

### pnpm v11 (current)

- **Released:** 2025-2026
- **Breaking: `pnpm` field in `package.json` no longer read** — config moved to `.npmrc` / `pnpm-workspace.yaml`
- **Build scripts blocked by default** — use `pnpm approve-builds` to allow specific packages
- New config commands: `pnpm config set --location project`

### pnpm v10 (previous)

- `pnpm` field in `package.json` was supported for config like `onlyBuiltDependencies`
- Build scripts were not blocked by default

## Tags

`#javascript` `#nodejs` `#package-manager` `#pnpm` `#build` `#docker` `#ci`
