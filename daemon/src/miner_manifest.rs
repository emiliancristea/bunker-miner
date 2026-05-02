use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::checksum::is_valid_sha256;
use crate::config::CONFIG_DIR_ENV;

pub const MINER_MANIFEST_PATH_ENV: &str = "BUNKER_MINER_MANIFEST_PATH";
const MANIFEST_FILE_NAME: &str = "miner-manifest.toml";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct MinerManifest {
    pub schema_version: u32,
    #[serde(default)]
    pub miners: Vec<MinerManifestEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MinerManifestEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub executable: String,
    pub sha256: String,
    pub source_url: String,
    #[serde(default)]
    pub archive_sha256: Option<String>,
    #[serde(default)]
    pub signature_url: Option<String>,
}

impl MinerManifest {
    pub fn parse_toml(input: &str) -> Result<Self> {
        let manifest: Self =
            toml::from_str(input).context("Failed to parse miner manifest TOML")?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let input = fs::read_to_string(path)
            .with_context(|| format!("Failed to read miner manifest {}", path.display()))?;
        Self::parse_toml(&input)
            .with_context(|| format!("Miner manifest {} failed validation", path.display()))
    }

    pub fn trusted_sha256(
        &self,
        name: &str,
        version: &str,
        executable: &str,
        platform: &str,
    ) -> Option<String> {
        self.miners
            .iter()
            .find(|entry| {
                entry.name.eq_ignore_ascii_case(name)
                    && entry.version == version
                    && entry.executable.eq_ignore_ascii_case(executable)
                    && platform_matches(&entry.platform, platform)
            })
            .map(|entry| entry.sha256.to_ascii_lowercase())
    }

    pub fn matching_entries(
        &self,
        name: &str,
        version: Option<&str>,
        platform: &str,
    ) -> Vec<MinerManifestEntry> {
        self.miners
            .iter()
            .filter(|entry| {
                entry.name.eq_ignore_ascii_case(name)
                    && version
                        .map(|version| entry.version == version)
                        .unwrap_or(true)
                    && platform_matches(&entry.platform, platform)
            })
            .cloned()
            .collect()
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != SUPPORTED_SCHEMA_VERSION {
            return Err(anyhow!(
                "Unsupported miner manifest schema_version {}; expected {}",
                self.schema_version,
                SUPPORTED_SCHEMA_VERSION
            ));
        }

        for entry in &self.miners {
            validate_required("name", &entry.name)?;
            validate_required("version", &entry.version)?;
            validate_required("platform", &entry.platform)?;
            validate_required("executable", &entry.executable)?;
            validate_required("source_url", &entry.source_url)?;
            validate_manifest_token("name", &entry.name)?;
            validate_manifest_token("version", &entry.version)?;
            validate_manifest_platform(&entry.platform)?;
            validate_manifest_token("executable", &entry.executable)?;

            if !is_valid_sha256(&entry.sha256) {
                return Err(anyhow!(
                    "Manifest entry {} {} has invalid executable sha256",
                    entry.name,
                    entry.version
                ));
            }

            if let Some(archive_sha256) = &entry.archive_sha256 {
                if !is_valid_sha256(archive_sha256) {
                    return Err(anyhow!(
                        "Manifest entry {} {} has invalid archive_sha256",
                        entry.name,
                        entry.version
                    ));
                }
            }

            if !entry.source_url.starts_with("https://") {
                return Err(anyhow!(
                    "Manifest entry {} {} source_url must use https",
                    entry.name,
                    entry.version
                ));
            }

            if let Some(signature_url) = &entry.signature_url {
                if !signature_url.starts_with("https://") {
                    return Err(anyhow!(
                        "Manifest entry {} {} signature_url must use https",
                        entry.name,
                        entry.version
                    ));
                }
            }
        }

        Ok(())
    }
}

pub fn trusted_sha256_for_binary(
    name: &str,
    version: &str,
    executable: &str,
) -> Result<Option<String>> {
    let platform = current_platform();

    for candidate in manifest_candidates() {
        if !candidate.path.exists() {
            if candidate.required {
                return Err(anyhow!(
                    "Configured miner manifest does not exist: {}",
                    candidate.path.display()
                ));
            }
            continue;
        }

        let manifest = MinerManifest::from_path(&candidate.path)?;
        if let Some(sha256) = manifest.trusted_sha256(name, version, executable, &platform) {
            return Ok(Some(sha256));
        }
    }

    Ok(None)
}

pub fn trusted_entry_for_miner(name: &str, version: Option<&str>) -> Result<MinerManifestEntry> {
    let platform = current_platform();
    let mut available_versions = Vec::new();
    let mut manifest_was_available = false;

    for candidate in manifest_candidates() {
        if !candidate.path.exists() {
            if candidate.required {
                return Err(anyhow!(
                    "Configured miner manifest does not exist: {}",
                    candidate.path.display()
                ));
            }
            continue;
        }

        manifest_was_available = true;
        let manifest = MinerManifest::from_path(&candidate.path)?;
        let matches = manifest.matching_entries(name, version, &platform);

        if matches.len() == 1 {
            return Ok(matches[0].clone());
        }

        if matches.len() > 1 {
            let versions = matches
                .iter()
                .map(|entry| entry.version.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!(
                "Miner manifest contains multiple matching entries for {name}; specify one of these versions: {versions}"
            ));
        }

        available_versions.extend(
            manifest
                .matching_entries(name, None, &platform)
                .into_iter()
                .map(|entry| entry.version),
        );
    }

    if !manifest_was_available {
        return Err(anyhow!(
            "No miner manifest is available. Set BUNKER_MINER_MANIFEST_PATH or place miner-manifest.toml in the managed config directory."
        ));
    }

    available_versions.sort();
    available_versions.dedup();

    match (version, available_versions.is_empty()) {
        (Some(version), false) => Err(anyhow!(
            "No manifest entry found for {name} {version} on {platform}; available versions: {}",
            available_versions.join(", ")
        )),
        (Some(version), true) => Err(anyhow!(
            "No manifest entry found for {name} {version} on {platform}"
        )),
        (None, false) => Err(anyhow!(
            "No unique manifest entry found for {name} on {platform}; specify one of these versions: {}",
            available_versions.join(", ")
        )),
        (None, true) => Err(anyhow!(
            "No manifest entry found for {name} on {platform}"
        )),
    }
}

pub fn current_platform() -> String {
    format!("{}-{}", env::consts::OS, env::consts::ARCH)
}

#[derive(Debug)]
struct ManifestCandidate {
    path: PathBuf,
    required: bool,
}

fn manifest_candidates() -> Vec<ManifestCandidate> {
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os(MINER_MANIFEST_PATH_ENV) {
        candidates.push(ManifestCandidate {
            path: PathBuf::from(path),
            required: true,
        });
    }

    let config_dir = match env::var_os(CONFIG_DIR_ENV) {
        Some(path) => Some(PathBuf::from(path)),
        None => dirs::config_dir().map(|path| path.join("bunker-miner")),
    };

    if let Some(config_dir) = config_dir {
        candidates.push(ManifestCandidate {
            path: config_dir.join(MANIFEST_FILE_NAME),
            required: false,
        });
    }

    candidates
}

fn validate_required(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("Manifest field {field} must not be empty"));
    }

    Ok(())
}

fn validate_manifest_token(field: &str, value: &str) -> Result<()> {
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(anyhow!(
            "Manifest field {field} may only contain ASCII letters, numbers, dots, underscores, and hyphens"
        ));
    }

    Ok(())
}

fn validate_manifest_platform(value: &str) -> Result<()> {
    if matches!(value, "*" | "any") {
        return Ok(());
    }

    validate_manifest_token("platform", value)
}

fn platform_matches(entry_platform: &str, current_platform: &str) -> bool {
    matches!(entry_platform, "*" | "any") || entry_platform.eq_ignore_ascii_case(current_platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn manifest_matches_current_platform_binary() {
        let manifest = MinerManifest::parse_toml(&format!(
            r#"
schema_version = 1

[[miners]]
name = "XMRig"
version = "6.20.0"
platform = "{}"
executable = "xmrig.exe"
sha256 = "{}"
source_url = "https://github.com/xmrig/xmrig/releases/download/v6.20.0/xmrig.zip"
"#,
            current_platform(),
            TEST_HASH
        ))
        .unwrap();

        assert_eq!(
            manifest.trusted_sha256("xmrig", "6.20.0", "xmrig.exe", &current_platform()),
            Some(TEST_HASH.to_string())
        );
    }

    #[test]
    fn manifest_rejects_invalid_sha256() {
        let error = MinerManifest::parse_toml(
            r#"
schema_version = 1

[[miners]]
name = "XMRig"
version = "6.20.0"
platform = "any"
executable = "xmrig.exe"
sha256 = "not-a-hash"
source_url = "https://github.com/xmrig/xmrig/releases/download/v6.20.0/xmrig.zip"
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("invalid executable sha256"));
    }

    #[test]
    fn manifest_rejects_non_https_source() {
        let error = MinerManifest::parse_toml(&format!(
            r#"
schema_version = 1

[[miners]]
name = "XMRig"
version = "6.20.0"
platform = "any"
executable = "xmrig.exe"
sha256 = "{}"
source_url = "http://example.com/xmrig.zip"
"#,
            TEST_HASH
        ))
        .unwrap_err();

        assert!(error.to_string().contains("source_url must use https"));
    }

    #[test]
    fn manifest_rejects_path_like_executable() {
        let error = MinerManifest::parse_toml(&format!(
            r#"
schema_version = 1

[[miners]]
name = "XMRig"
version = "6.20.0"
platform = "any"
executable = "../xmrig.exe"
sha256 = "{}"
source_url = "https://github.com/xmrig/xmrig/releases/download/v6.20.0/xmrig.zip"
"#,
            TEST_HASH
        ))
        .unwrap_err();

        assert!(error.to_string().contains("Manifest field executable"));
    }
}
