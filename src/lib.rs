#![warn(clippy::all, clippy::pedantic)]

use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result};

const BINARY_NAME: &str = "deps-lsp";
const GITHUB_REPO: &str = "bug-ops/deps-lsp";

/// Computes the lowercase hex-encoded SHA-256 digest of a file's contents.
fn sha256_hex(path: &str) -> Result<String> {
    let bytes = fs::read(path).map_err(|err| format!("failed to read '{path}': {err}"))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
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

            // Downloaded uncompressed first so the raw archive bytes can be
            // hashed; download_file extracts in place and doesn't expose
            // the archive contents otherwise.
            zed::download_file(
                &asset.download_url,
                &archive_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|err| format!("failed to download file: {err}"))?;

            let actual_checksum = sha256_hex(&archive_path)?;
            if !actual_checksum.eq_ignore_ascii_case(&expected_checksum) {
                fs::remove_file(&archive_path).ok();
                return Err(format!(
                    "checksum mismatch for {asset_name}: expected {expected_checksum}, got {actual_checksum}"
                ));
            }

            if let Err(err) = zed::download_file(&asset.download_url, &version_dir, file_type) {
                fs::remove_dir_all(&version_dir).ok();
                return Err(format!("failed to extract file: {err}"));
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
