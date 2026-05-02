use anyhow::{anyhow, bail, Context, Result};
use futures_util::StreamExt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tracing::info;
use uuid::Uuid;
use zip::ZipArchive;

use crate::checksum::sha256_file;
use crate::miner_manifest::{self, MinerManifestEntry};

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct MinerInstallResult {
    pub name: String,
    pub version: String,
    pub executable_path: PathBuf,
    pub executable_sha256: String,
    pub source_url: String,
}

#[derive(Debug, Clone)]
pub struct MinerInstaller {
    binaries_dir: PathBuf,
    http_client: reqwest::Client,
}

impl MinerInstaller {
    pub fn new(binaries_dir: PathBuf) -> Self {
        Self {
            binaries_dir,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn install_from_manifest(
        &self,
        name: &str,
        version: Option<&str>,
        force: bool,
    ) -> Result<MinerInstallResult> {
        let entry = miner_manifest::trusted_entry_for_miner(name, version)?;
        self.install_entry_from_network(&entry, force).await
    }

    async fn install_entry_from_network(
        &self,
        entry: &MinerManifestEntry,
        force: bool,
    ) -> Result<MinerInstallResult> {
        validate_archive_source(entry)?;
        let install_dir = self.install_dir(entry)?;
        let temp_dir = tempfile::tempdir_in(&install_dir).with_context(|| {
            format!(
                "Failed to create installer temp dir in {}",
                install_dir.display()
            )
        })?;
        let archive_path = temp_dir.path().join("miner-archive.zip");

        download_archive(&self.http_client, entry, &archive_path).await?;
        self.install_verified_archive(entry, &archive_path, force)
            .await
    }

    pub async fn install_verified_archive(
        &self,
        entry: &MinerManifestEntry,
        archive_path: &Path,
        force: bool,
    ) -> Result<MinerInstallResult> {
        let archive_sha256 = entry.archive_sha256.as_deref().ok_or_else(|| {
            anyhow!(
                "Manifest entry {} {} cannot be installed because archive_sha256 is missing",
                entry.name,
                entry.version
            )
        })?;

        let actual_archive_sha256 = sha256_file(archive_path).await?;
        if actual_archive_sha256 != archive_sha256.to_ascii_lowercase() {
            bail!(
                "Archive SHA-256 mismatch for {} {}: expected {}, got {}",
                entry.name,
                entry.version,
                archive_sha256,
                actual_archive_sha256
            );
        }

        let install_dir = self.install_dir(entry)?;
        let target_path = install_dir.join(&entry.executable);
        let expected_executable_sha256 = entry.sha256.to_ascii_lowercase();

        if target_path.exists() {
            let existing_sha256 = sha256_file(&target_path).await?;
            if existing_sha256 == expected_executable_sha256 {
                return Ok(MinerInstallResult {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    executable_path: target_path,
                    executable_sha256: existing_sha256,
                    source_url: entry.source_url.clone(),
                });
            }

            if !force {
                bail!(
                    "Refusing to replace existing miner executable {} because its SHA-256 is {}; rerun with force=true to replace it",
                    target_path.display(),
                    existing_sha256
                );
            }
        }

        let temp_dir = tempfile::tempdir_in(&install_dir).with_context(|| {
            format!(
                "Failed to create installer staging dir in {}",
                install_dir.display()
            )
        })?;
        let staged_path = temp_dir.path().join(&entry.executable);
        extract_zip_executable(archive_path, &entry.executable, &staged_path)?;

        let actual_executable_sha256 = sha256_file(&staged_path).await?;
        if actual_executable_sha256 != expected_executable_sha256 {
            bail!(
                "Executable SHA-256 mismatch for {} {}: expected {}, got {}",
                entry.name,
                entry.version,
                expected_executable_sha256,
                actual_executable_sha256
            );
        }

        set_executable_permissions(&staged_path)?;
        replace_executable(&staged_path, &target_path, force)?;

        info!(
            "Installed verified miner {} {} to {}",
            entry.name,
            entry.version,
            target_path.display()
        );

        Ok(MinerInstallResult {
            name: entry.name.clone(),
            version: entry.version.clone(),
            executable_path: target_path,
            executable_sha256: actual_executable_sha256,
            source_url: entry.source_url.clone(),
        })
    }

    fn install_dir(&self, entry: &MinerManifestEntry) -> Result<PathBuf> {
        let path = self.binaries_dir.join(entry.name.to_ascii_lowercase());
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create miner install dir {}", path.display()))?;
        Ok(path)
    }
}

async fn download_archive(
    http_client: &reqwest::Client,
    entry: &MinerManifestEntry,
    archive_path: &Path,
) -> Result<()> {
    let response = http_client
        .get(&entry.source_url)
        .send()
        .await
        .with_context(|| format!("Failed to request {}", entry.source_url))?
        .error_for_status()
        .with_context(|| format!("Miner archive request failed for {}", entry.source_url))?;

    if let Some(content_length) = response.content_length() {
        if content_length > MAX_ARCHIVE_BYTES {
            bail!(
                "Refusing to download {} because content length {} exceeds {} bytes",
                entry.source_url,
                content_length,
                MAX_ARCHIVE_BYTES
            );
        }
    }

    let mut output = tokio::fs::File::create(archive_path)
        .await
        .with_context(|| format!("Failed to create {}", archive_path.display()))?;
    let mut downloaded = 0_u64;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.with_context(|| format!("Failed while downloading {}", entry.source_url))?;
        downloaded += chunk.len() as u64;
        if downloaded > MAX_ARCHIVE_BYTES {
            bail!(
                "Refusing to download {} because archive exceeds {} bytes",
                entry.source_url,
                MAX_ARCHIVE_BYTES
            );
        }
        output.write_all(&chunk).await?;
    }

    output.flush().await?;

    if downloaded == 0 {
        bail!(
            "Downloaded miner archive from {} was empty",
            entry.source_url
        );
    }

    Ok(())
}

fn validate_archive_source(entry: &MinerManifestEntry) -> Result<()> {
    let url = reqwest::Url::parse(&entry.source_url)
        .with_context(|| format!("Manifest source_url is invalid: {}", entry.source_url))?;

    if url.scheme() != "https" {
        bail!("Manifest source_url must use https: {}", entry.source_url);
    }

    if !url.path().to_ascii_lowercase().ends_with(".zip") {
        bail!(
            "Only zip miner archives are currently supported for verified acquisition: {}",
            entry.source_url
        );
    }

    Ok(())
}

fn extract_zip_executable(
    archive_path: &Path,
    executable_name: &str,
    staged_path: &Path,
) -> Result<()> {
    let archive_file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(archive_file)
        .with_context(|| format!("Failed to read zip archive {}", archive_path.display()))?;
    let matching_indices = matching_zip_entries(&mut archive, executable_name)?;

    if matching_indices.is_empty() {
        bail!(
            "Archive {} does not contain expected executable {}",
            archive_path.display(),
            executable_name
        );
    }
    if matching_indices.len() > 1 {
        bail!(
            "Archive {} contains multiple files named {}; refusing ambiguous extraction",
            archive_path.display(),
            executable_name
        );
    }

    let mut input = archive.by_index(matching_indices[0])?;
    if input.size() > MAX_EXECUTABLE_BYTES {
        bail!(
            "Refusing to extract {} because uncompressed size {} exceeds {} bytes",
            executable_name,
            input.size(),
            MAX_EXECUTABLE_BYTES
        );
    }

    let parent = staged_path.parent().ok_or_else(|| {
        anyhow!(
            "Installer staging path has no parent directory: {}",
            staged_path.display()
        )
    })?;
    fs::create_dir_all(parent)?;

    let mut output = File::create(staged_path).with_context(|| {
        format!(
            "Failed to create staged executable {}",
            staged_path.display()
        )
    })?;
    let copied = copy_limited(&mut input, &mut output, MAX_EXECUTABLE_BYTES)?;
    if copied == 0 {
        bail!("Extracted executable {executable_name} is empty");
    }
    output.flush()?;

    Ok(())
}

fn matching_zip_entries<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    executable_name: &str,
) -> Result<Vec<usize>> {
    let mut matching_indices = Vec::new();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        let Some(enclosed_name) = file.enclosed_name() else {
            continue;
        };

        let Some(file_name) = enclosed_name.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name.eq_ignore_ascii_case(executable_name) {
            matching_indices.push(index);
        }
    }

    Ok(matching_indices)
}

fn copy_limited(input: &mut impl Read, output: &mut impl Write, max_bytes: u64) -> Result<u64> {
    let mut limited = input.take(max_bytes + 1);
    let copied = std::io::copy(&mut limited, output)?;
    if copied > max_bytes {
        bail!("Refusing to extract executable because it exceeds {max_bytes} bytes");
    }
    Ok(copied)
}

fn replace_executable(staged_path: &Path, target_path: &Path, force: bool) -> Result<()> {
    let backup_path = target_path.with_extension(format!(
        "{}install-backup-{}",
        target_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default(),
        Uuid::new_v4()
    ));

    let mut backup_created = false;
    if target_path.exists() {
        if !force {
            bail!(
                "Refusing to replace existing miner executable {} without force=true",
                target_path.display()
            );
        }

        fs::rename(target_path, &backup_path).with_context(|| {
            format!(
                "Failed to move existing miner executable {} to backup {}",
                target_path.display(),
                backup_path.display()
            )
        })?;
        backup_created = true;
    }

    match fs::rename(staged_path, target_path) {
        Ok(()) => {
            if backup_created {
                let _ = fs::remove_file(&backup_path);
            }
            Ok(())
        }
        Err(error) => {
            if backup_created {
                let _ = fs::rename(&backup_path, target_path);
            }
            Err(error).with_context(|| {
                format!(
                    "Failed to move verified miner executable into {}",
                    target_path.display()
                )
            })
        }
    }
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::sha256_bytes;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    const TEST_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test]
    async fn installer_extracts_verified_zip_archive() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("xmrig.zip");
        let executable_contents = b"verified executable fixture";
        create_zip(&archive_path, "xmrig-6.20.0/xmrig.exe", executable_contents);

        let entry = test_entry(
            sha256_bytes(executable_contents),
            sha256_file(&archive_path).await.unwrap(),
        );
        let installer = MinerInstaller::new(temp_dir.path().join("binaries"));

        let result = installer
            .install_verified_archive(&entry, &archive_path, false)
            .await
            .unwrap();

        assert_eq!(result.name, "XMRig");
        assert_eq!(result.version, "6.20.0");
        assert_eq!(
            fs::read(result.executable_path).unwrap(),
            executable_contents
        );
    }

    #[tokio::test]
    async fn installer_rejects_archive_checksum_mismatch() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("xmrig.zip");
        create_zip(&archive_path, "xmrig.exe", b"verified executable fixture");

        let entry = test_entry(
            sha256_bytes(b"verified executable fixture"),
            TEST_HASH.to_string(),
        );
        let installer = MinerInstaller::new(temp_dir.path().join("binaries"));

        let error = installer
            .install_verified_archive(&entry, &archive_path, false)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("Archive SHA-256 mismatch"));
    }

    #[tokio::test]
    async fn installer_refuses_existing_mismatch_without_force() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("xmrig.zip");
        let executable_contents = b"verified executable fixture";
        create_zip(&archive_path, "xmrig.exe", executable_contents);

        let entry = test_entry(
            sha256_bytes(executable_contents),
            sha256_file(&archive_path).await.unwrap(),
        );
        let install_dir = temp_dir.path().join("binaries").join("xmrig");
        fs::create_dir_all(&install_dir).unwrap();
        fs::write(install_dir.join("xmrig.exe"), b"old executable").unwrap();

        let installer = MinerInstaller::new(temp_dir.path().join("binaries"));
        let error = installer
            .install_verified_archive(&entry, &archive_path, false)
            .await
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("Refusing to replace existing miner executable"));
    }

    fn test_entry(executable_sha256: String, archive_sha256: String) -> MinerManifestEntry {
        MinerManifestEntry {
            name: "XMRig".to_string(),
            version: "6.20.0".to_string(),
            platform: "any".to_string(),
            executable: "xmrig.exe".to_string(),
            sha256: executable_sha256,
            source_url: "https://example.com/xmrig.zip".to_string(),
            archive_sha256: Some(archive_sha256),
            signature_url: None,
        }
    }

    fn create_zip(archive_path: &Path, entry_name: &str, contents: &[u8]) {
        let file = File::create(archive_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file(entry_name, options).unwrap();
        zip.write_all(contents).unwrap();
        zip.finish().unwrap();
    }
}
