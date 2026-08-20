# Changelog

All notable changes to this project will be documented in this file.

## [0.1.7] - 2026-08-20

### Added

- C# / NuGet language support, matching `deps-lsp` 0.10.0's new NuGet
  ecosystem: `.csproj` files via the `C# Project File` language, and
  `.fsproj` / `.vbproj` / `Directory.Packages.props` via the `MSBuild File`
  language (both provided by the community C# extension)

## [0.1.6] - 2026-08-13

### Added

- Linux musl binary support (`deps-lsp` is now resolved as a statically-linked
  `unknown-linux-musl` target, which runs on both glibc and musl distros)
- SHA-256 checksum verification of the downloaded `deps-lsp` archive against
  the `.sha256` sidecar published on each GitHub release

### Changed

- Linux downloads now request the `unknown-linux-musl` asset instead of
  `unknown-linux-gnu`

## [0.1.5] - 2026-02-23

### Added

- Swift language support (Swift Package Manager)
- PHP language support (Composer)

## [0.1.4] - 2026-02-23

### Added

- XML language support (Maven pom.xml)
- Java language support
- Groovy language support (Gradle build files)
- Kotlin language support (Gradle Kotlin DSL)

## [0.1.3] - 2026-02-16

### Added

- Dart language support
- YAML language support

## [0.1.2] - 2025-12-28

### Added

- Ruby language support

## [0.1.1] - 2025-12-22

### Added

- Go Mod language support

### Changed

- Update extension metadata

## [0.1.0] - 2025-12-15

### Added

- Initial release
- TOML and JSON language support
- Auto-download binary from GitHub releases
- Platform-specific binary resolution (macOS, Linux, Windows)
- Old version cleanup
