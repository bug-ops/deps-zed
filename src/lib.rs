#![warn(clippy::all, clippy::pedantic)]

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path};
use zed_extension_api::{self as zed, LanguageServerId, Result};

const BINARY_NAME: &str = "deps-lsp";
const GITHUB_REPO: &str = "bug-ops/deps-lsp";

/// Computes the lowercase hex-encoded SHA-256 digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// Extracts a 64-character hex SHA-256 digest from a checksum sidecar file's contents.
///
/// Linux/macOS sidecars use plain `sha256sum` format (`<hex>  <filename>`), but Windows
/// sidecars are `CertUtil -hashfile` output, which wraps the digest in extra lines of
/// prose. Scanning for the hex token handles both without assuming it's the first word.
fn parse_sha256_sidecar(content: &str) -> Result<String> {
    content
        .split_whitespace()
        .find(|token| token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_string)
        .ok_or_else(|| "malformed checksum file: no 64-character hex digest found".to_string())
}

/// Extracts a gzip-compressed tar archive into `dest_dir`.
///
/// Entries are copied one by one (rather than via `tar::Archive::unpack`) because `unpack`
/// unconditionally calls `set_permissions`, which is unimplemented under WASI and would fail
/// every extraction on Zed's wasm32-wasip2 host. Each entry's path is checked to reject
/// traversal outside `dest_dir` instead of relying on `unpack`'s default behavior of silently
/// skipping such entries.
fn extract_tar_gz(archive_bytes: &[u8], dest_dir: &str) -> Result<()> {
    let mut archive = tar::Archive::new(GzDecoder::new(archive_bytes));
    let entries = archive
        .entries()
        .map_err(|err| format!("failed to read archive entries: {err}"))?;

    for entry in entries {
        let mut entry = entry.map_err(|err| format!("failed to read archive entry: {err}"))?;
        let relative_path = entry
            .path()
            .map_err(|err| format!("failed to read entry path: {err}"))?
            .into_owned();

        if relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(format!(
                "archive entry has unsafe path: {}",
                relative_path.display()
            ));
        }

        let dest_path = Path::new(dest_dir).join(&relative_path);

        match entry.header().entry_type() {
            tar::EntryType::Directory => {
                fs::create_dir_all(&dest_path).map_err(|err| {
                    format!(
                        "failed to create directory '{}': {err}",
                        dest_path.display()
                    )
                })?;
            }
            tar::EntryType::Regular => {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(|err| {
                        format!("failed to create directory '{}': {err}", parent.display())
                    })?;
                }
                let mut out_file = fs::File::create(&dest_path)
                    .map_err(|err| format!("failed to create '{}': {err}", dest_path.display()))?;
                io::copy(&mut entry, &mut out_file)
                    .map_err(|err| format!("failed to write '{}': {err}", dest_path.display()))?;
            }
            other => {
                return Err(format!(
                    "unsupported archive entry type {other:?} for '{}'",
                    relative_path.display()
                ));
            }
        }
    }

    Ok(())
}

/// Extracts an already-downloaded and checksum-verified archive into `dest_dir`.
fn extract_archive(
    archive_bytes: &[u8],
    dest_dir: &str,
    file_type: zed::DownloadedFileType,
) -> Result<()> {
    match file_type {
        zed::DownloadedFileType::GzipTar => extract_tar_gz(archive_bytes, dest_dir)?,
        zed::DownloadedFileType::Zip => {
            let mut archive = zip::ZipArchive::new(io::Cursor::new(archive_bytes))
                .map_err(|err| format!("failed to open zip archive: {err}"))?;
            archive
                .extract(dest_dir)
                .map_err(|err| format!("failed to extract archive: {err}"))?;
        }
        zed::DownloadedFileType::Gzip | zed::DownloadedFileType::Uncompressed => {
            return Err(format!(
                "unsupported archive type for extraction: {file_type:?}"
            ));
        }
    }

    Ok(())
}

struct DepsExtension {
    cached_binary_path: Option<String>,
}

impl DepsExtension {
    /// Returns the path to the `deps-lsp` binary.
    ///
    /// Lookup order:
    /// 1. Cached path from previous invocation
    /// 2. System PATH via `worktree.which()`
    /// 3. Download from GitHub releases
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // Check cached path
        if let Some(path) = &self.cached_binary_path
            && fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        // Check system PATH
        if let Some(path) = worktree.which(BINARY_NAME) {
            return Ok(path);
        }

        // Download from GitHub releases
        self.download_binary(language_server_id)
    }

    fn download_binary(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();

        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X86 => "x86",
            zed::Architecture::X8664 => "x86_64",
        };
        let (os_suffix, os_short, bin_name, file_type) = match platform {
            zed::Os::Mac => (
                "apple-darwin.tar.gz",
                "macos",
                BINARY_NAME.to_string(),
                zed::DownloadedFileType::GzipTar,
            ),
            // musl binaries are statically linked and run on both glibc
            // and musl distros; zed_extension_api has no way to detect
            // the host libc, so musl covers both cases unconditionally.
            zed::Os::Linux => (
                "unknown-linux-musl.tar.gz",
                "linux",
                BINARY_NAME.to_string(),
                zed::DownloadedFileType::GzipTar,
            ),
            zed::Os::Windows => (
                "pc-windows-msvc.zip",
                "windows",
                format!("{BINARY_NAME}.exe"),
                zed::DownloadedFileType::Zip,
            ),
        };
        let asset_name = format!("{BINARY_NAME}-{arch_str}-{os_suffix}");

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        let checksum_name = format!("{asset_name}.sha256");
        let checksum_asset = release
            .assets
            .iter()
            .find(|asset| asset.name == checksum_name)
            .ok_or_else(|| format!("no checksum asset found matching {checksum_name:?}"))?;

        let version_dir = format!("{BINARY_NAME}-{}-{arch_str}-{os_short}", release.version);

        fs::create_dir_all(&version_dir)
            .map_err(|err| format!("failed to create directory '{version_dir}': {err}"))?;

        let binary_path = format!("{version_dir}/{bin_name}");

        // Download if binary doesn't exist
        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            let archive_path = format!("{version_dir}/{asset_name}");
            let checksum_path = format!("{archive_path}.sha256");

            zed::download_file(
                &checksum_asset.download_url,
                &checksum_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|err| format!("failed to download checksum: {err}"))?;

            let checksum_content = fs::read_to_string(&checksum_path)
                .map_err(|err| format!("failed to read checksum file: {err}"))?;
            fs::remove_file(&checksum_path).ok();

            let expected_checksum = parse_sha256_sidecar(&checksum_content)?;

            // Downloaded uncompressed so the raw archive bytes can be hashed and
            // verified before extraction, rather than trusting a second, unverified fetch.
            zed::download_file(
                &asset.download_url,
                &archive_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|err| format!("failed to download file: {err}"))?;

            // Read once and reuse the same in-memory bytes for both hashing and extraction,
            // so a file changed on disk between the two steps can't slip past verification.
            let archive_bytes = fs::read(&archive_path)
                .map_err(|err| format!("failed to read '{archive_path}': {err}"))?;

            let actual_checksum = sha256_hex(&archive_bytes);
            if !actual_checksum.eq_ignore_ascii_case(&expected_checksum) {
                fs::remove_file(&archive_path).ok();
                return Err(format!(
                    "checksum mismatch for {asset_name}: expected {expected_checksum}, got {actual_checksum}"
                ));
            }

            if let Err(err) = extract_archive(&archive_bytes, &version_dir, file_type) {
                fs::remove_dir_all(&version_dir).ok();
                return Err(err);
            }
            fs::remove_file(&archive_path).ok();

            if let Err(err) = zed::make_file_executable(&binary_path) {
                fs::remove_dir_all(&version_dir).ok();
                return Err(err);
            }

            // Clean up old versions
            Self::cleanup_old_versions(&version_dir);
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn cleanup_old_versions(current_version_dir: &str) {
        let Ok(entries) = fs::read_dir(".") else {
            return;
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };

            // Remove old deps-lsp-* directories
            if name_str.starts_with(BINARY_NAME) && name_str != current_version_dir {
                fs::remove_dir_all(entry.path()).ok();
            }
        }
    }
}

impl zed::Extension for DepsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: vec!["--stdio".into()],
            env: Vec::default(),
        })
    }
}

zed::register_extension!(DepsExtension);

#[cfg(test)]
mod tests {
    use super::{extract_tar_gz, fs};
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn build_tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("deps-zed-test-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn extract_tar_gz_writes_regular_files() {
        let dir = scratch_dir("extract");
        let archive_bytes = build_tar_gz(&[("bin/tool", b"hello")]);

        extract_tar_gz(&archive_bytes, dir.to_str().unwrap()).unwrap();

        assert_eq!(fs::read(dir.join("bin/tool")).unwrap(), b"hello");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extract_tar_gz_accepts_current_dir_prefixed_entries() {
        let dir = scratch_dir("curdir");
        let archive_bytes = build_tar_gz(&[("./deps-lsp", b"binary")]);

        extract_tar_gz(&archive_bytes, dir.to_str().unwrap()).unwrap();

        assert_eq!(fs::read(dir.join("deps-lsp")).unwrap(), b"binary");
        fs::remove_dir_all(&dir).ok();
    }

    /// Builds a single-entry tar.gz whose entry name is written into the header directly,
    /// bypassing `tar::Header::set_path`'s own `..` validation, to simulate a malicious upstream
    /// archive that `extract_tar_gz` must reject on its own.
    fn build_malicious_tar_gz(name: &str, data: &[u8]) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        {
            let name_bytes = name.as_bytes();
            let raw = header.as_mut_bytes();
            raw[..name_bytes.len()].copy_from_slice(name_bytes);
        }
        header.set_cksum();

        let mut tar_bytes = Vec::new();
        tar_bytes.extend_from_slice(header.as_bytes());
        tar_bytes.extend_from_slice(data);
        tar_bytes.resize(tar_bytes.len().div_ceil(512) * 512, 0);
        tar_bytes.extend(std::iter::repeat_n(0u8, 1024));

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_bytes).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn extract_tar_gz_rejects_path_traversal() {
        let dir = scratch_dir("traversal");
        let archive_bytes = build_malicious_tar_gz("../evil", b"pwn");

        let result = extract_tar_gz(&archive_bytes, dir.to_str().unwrap());

        assert!(result.is_err());
        assert!(!dir.join("../evil").exists());
        fs::remove_dir_all(&dir).ok();
    }
}
