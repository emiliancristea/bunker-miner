use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;

pub async fn sha256_file(path: &Path) -> Result<String> {
    let data = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read {}", path.display()))?;
    Ok(sha256_bytes(&data))
}

pub fn sha256_reader(mut reader: impl Read) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format_digest(hasher.finalize()))
}

pub fn sha256_bytes(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    format_digest(digest)
}

pub fn parse_sha256_value(value: &str) -> Result<String> {
    let Some(candidate) = value.split_whitespace().find(|part| is_valid_sha256(part)) else {
        return Err(anyhow!(
            "Checksum value must contain a 64-character lowercase or uppercase hex SHA-256 digest"
        ));
    };

    Ok(candidate.to_ascii_lowercase())
}

pub fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn format_digest(digest: impl AsRef<[u8]>) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest.as_ref() {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256_from_sidecar_style_line() {
        let digest = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        assert_eq!(
            parse_sha256_value(&format!("{digest}  xmrig.exe")).unwrap(),
            digest.to_ascii_lowercase()
        );
    }

    #[test]
    fn hashes_bytes_and_reader_consistently() {
        let data = b"bunker miner checksum fixture";

        assert_eq!(
            sha256_bytes(data),
            sha256_reader(&data[..]).expect("reader hash should succeed")
        );
    }
}
