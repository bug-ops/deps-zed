# Deps

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)

Zed extension for intelligent dependency management in `Cargo.toml`, `package.json`, and more.

![Deps extension in action](assets/img.png)

## Features

- **Version Hints** — Inline status indicators (✅ up-to-date, ❌ outdated)
- **Lock File Support** — Resolved versions from Cargo.lock, package-lock.json, poetry.lock, uv.lock
- **Hover Information** — Version list with resolved version from lock file
- **Diagnostics** — Warnings for outdated, yanked, or unknown packages
- **Code Actions** — Quick version updates via `Cmd+.`
- **Autocomplete** — Package names, versions, and feature flags

## Installation

Install **Deps** from Zed Extensions:

1. Open Zed
2. Press `Cmd+Shift+X` to open Extensions
3. Search for "Deps"
4. Click Install

## Configuration

Configure in Zed settings (`Cmd+,`):

```json
{
  "lsp": {
    "deps-lsp": {
      "initialization_options": {
        "inlay_hints": {
          "enabled": true,
          "up_to_date_text": "✅",
          "needs_update_text": "❌ {}"
        },
        "diagnostics": {
          "outdated_severity": "hint",
          "unknown_severity": "warning"
        }
      }
    }
  }
}
```

## Supported Files

| Ecosystem | Manifest | Lock File |
|-----------|----------|-----------|
| Rust | `Cargo.toml` | `Cargo.lock` |
| npm | `package.json` | `package-lock.json` |
| Python | `pyproject.toml` | `poetry.lock`, `uv.lock` |

## Development

> [!NOTE]
> This crate compiles to WASM and is distributed via Zed Extensions marketplace.

```bash
# Build WASM extension
cargo build -p deps-zed --target wasm32-wasip1
```

## License

[MIT](../../LICENSE)
