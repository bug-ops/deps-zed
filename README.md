# Deps

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)

Zed extension for [deps-lsp](https://github.com/bug-ops/deps-lsp) — intelligent dependency management in `Cargo.toml`, `package.json`, `Gemfile`, and more.

![Deps extension in action](assets/img.png)

## Features

- **Version Hints** — Inline status indicators (✅ up-to-date, ❌ outdated)
- **Lock File Support** — Resolved versions from Cargo.lock, package-lock.json, Gemfile.lock, etc.
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
| Go | `go.mod` | `go.sum` |
| Ruby | `Gemfile` | `Gemfile.lock` |

## Development

### Prerequisites

Install Rust via [rustup](https://rustup.rs/) and add the WASM target:

```bash
rustup target add wasm32-wasip1
```

### Build WASM Extension

```bash
cd crates/deps-zed
cargo build --release --target wasm32-wasip1
```

### Install as Dev Extension

1. Build the extension (see above)
2. Open Zed
3. Run command: `zed: install dev extension`
4. Select the `crates/deps-zed` directory

The extension will load from your local build. To see logs, run Zed from terminal:

```bash
zed --foreground
```

### Testing with Gemfile

1. Install the [Ruby extension](https://zed.dev/extensions/ruby) in Zed
2. Open a project with `Gemfile` and `Gemfile.lock`
3. Verify inlay hints and hover information appear

## License

[MIT](../../LICENSE)
