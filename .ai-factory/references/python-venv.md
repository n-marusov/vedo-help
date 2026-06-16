# Python Virtual Environment Reference (.venv)

> Source: `.venv/` (project virtual environment)
> Created: 2026-06-16
> Updated: 2026-06-16

## Overview

This project uses a local Python virtual environment located at `.venv/` in the project root. The environment was created with **uv** 0.8.3 and runs **Python 3.13.5** on **Windows**. All Python-related commands (embedding service, scripts, tooling) must be executed within this environment.

## Core Concepts

- **Virtual environment location:** `D:\Projects\vedo-help\.venv\` (project-relative: `.venv/`)
- **Python binary:** managed by uv; the `home` points to `C:\Users\nmaru\AppData\Roaming\uv\python\cpython-3.13.5-windows-x86_64-none`
- **Package manager:** `uv` is recommended over `pip` for all operations
- **Environment marker:** `prompt = vedo-help` (shell prompt prefix when activated)
- **System site-packages:** isolated (`include-system-site-packages = false`)

## Activation on Windows

Always use **cmd** or **PowerShell** on this system (not sh/bash — the agent runs in `sh` by default).

### cmd

```cmd
.venv\Scripts\activate.bat
```

### PowerShell

```powershell
.venv\Scripts\Activate.ps1
```

**Important for the Zed coding agent:** The agent's default shell is `sh`, but this is a Windows system. When running Python commands through the terminal tool, always use the full path to the Python executable inside the venv to avoid activation issues:

```sh
# From project root — use the absolute path to the venv Python
.venv/Scripts/python.exe --version
.venv/Scripts/python.exe -m pip list
.venv/Scripts/python.exe -m uvicorn embedding.src.main:app
```

## Running the Embedding Service

The Python embedding service lives in `embedding/` and runs via FastAPI/uvicorn:

```sh
# Start the embedding service with the venv Python
.venv/Scripts/python.exe -m uvicorn embedding.src.main:app --host 0.0.0.0 --port 8001 --reload
```

Or if using `uv`:

```sh
uv run uvicorn embedding.src.main:app --host 0.0.0.0 --port 8001
```

## Dependency Management

### Install dependencies (from requirements.txt)

```sh
# Using uv (recommended)
uv pip install -r embedding/requirements.txt

# Using pip
.venv/Scripts/python.exe -m pip install -r embedding/requirements.txt
```

### Install a new package

```sh
uv pip install <package-name>
```

### Freeze current packages

```sh
.venv/Scripts/python.exe -m pip freeze > requirements.txt
```

## Running Python Scripts

```sh
.venv/Scripts/python.exe scripts/backup.py    # if a Python script exists
.venv/Scripts/python.exe -c "print('hello')"  # inline
```

## Common Pitfalls

- **Do NOT use `python` or `python3` bare** — this will invoke the system Python, not the venv Python
- **The agent's `sh` shell cannot source activate scripts** — always use the full `.venv/Scripts/python.exe` path
- **Spaces in paths:** If the project root contains spaces, quote the path: `".venv/Scripts/python.exe"`
- **Empty `Scripts/` directory:** If `.venv/Scripts/` appears empty, the environment may be incomplete. Run `uv venv` to recreate it or `uv sync` to populate it
- **Windows line endings:** Python scripts checked out with CRLF work fine, but shebangs in `.py` files are ignored on Windows anyway

## Version Notes

| Aspect | Value |
|--------|-------|
| Python version | 3.13.5 |
| uv version | 0.8.3 |
| OS | Windows (x86_64) |
| venv location | `.venv/` |
| Environment manager | uv |
