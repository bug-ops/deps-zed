# Deps

[![Zed Extensions](https://img.shields.io/badge/Zed-Extensions-blue?logo=zedindustries)](https://zed.dev/extensions/deps-language-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Zed editor extension for [deps-lsp](https://github.com/bug-ops/deps-lsp) — intelligent dependency insights across package ecosystems.

![Deps extension in action](assets/img.png)

## Features

- **Version Hints** — Inline status indicators (up-to-date / outdated)
- **Hover Information** — Version list with resolved version from lock file
- **Diagnostics** — Warnings for outdated, yanked, or unknown packages
- **Code Actions** — Quick version updates via `Cmd+.`
- **Autocomplete** — Package names, versions, and feature flags

## Supported Ecosystems

| Ecosystem | Manifest |
|-----------|----------|
| Rust | `Cargo.toml` |
| Node.js | `package.json` |
| Go | `go.mod` |
| Ruby | `Gemfile` |
| Dart / Flutter | `pubspec.yaml` |
| GitHub Actions / Docker Compose | YAML files |
| Maven | `pom.xml` |
| Java | build configs |
| Gradle | `build.gradle` |
| Gradle Kotlin DSL | `build.gradle.kts` |

## Installation

1. Open Zed
2. Press `Cmd+Shift+X` to open Extensions
3. Search for **Deps**
4. Click Install

## How It Works

The extension launches `deps-lsp` as a language server. On first use, it resolves the binary in this order:

1. Cached path from a previous run
2. System PATH (`deps-lsp` executable)
3. Auto-download from [GitHub releases](https://github.com/bug-ops/deps-lsp/releases) (platform-specific archive)

Old downloaded versions are cleaned up automatically after each update.

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

## License

[MIT](LICENSE)
