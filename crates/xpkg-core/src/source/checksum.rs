//! Checksum computation and verification for source files.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256, Sha512};

use crate::XpkgError;

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgo {
    Sha256,
    Sha512,
}

/// Compute the SHA-256 hex digest of a file.
pub fn compute_sha256(path: &Path) -> Result<String, XpkgError> {
    compute_digest::<Sha256>(path)
}

/// Compute the SHA-512 hex digest of a file.
pub fn compute_sha512(path: &Path) -> Result<String, XpkgError> {
    compute_digest::<Sha512>(path)
}

/// Verify a file's checksum against an expected hex digest.
///
/// Returns `Ok(())` if the checksum matches, or if `expected` is `"SKIP"`.
pub fn verify_checksum(path: &Path, expected: &str, algo: ChecksumAlgo) -> Result<(), XpkgError> {
    if expected == "SKIP" {
        tracing::debug!(path = %path.display(), "checksum verification skipped");
        return Ok(());
    }

    let actual = match algo {
        ChecksumAlgo::Sha256 => compute_sha256(path)?,
        ChecksumAlgo::Sha512 => compute_sha512(path)?,
    };

    if actual != expected.to_lowercase() {
        return Err(XpkgError::ChecksumMismatch(format!(
            "{}: expected {}, got {}",
            path.display(),
            expected,
            actual,
        )));
    }

    tracing::debug!(path = %path.display(), algo = ?algo, "checksum verified");
    Ok(())
}

/// Generic digest computation over a file.
fn compute_digest<D: Digest>(path: &Path) -> Result<String, XpkgError> {
    let mut file = File::open(path).map_err(|e| {
        XpkgError::Io(std::io::Error::new(
            e.kind(),
            format!("failed to open {}: {e}", path.display()),
        ))
    })?;

    let mut hasher = D::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let result = hasher.finalize();
    Ok(result.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello world\n").unwrap();

        let hash = compute_sha256(&path).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex digest is 64 chars
    }

    #[test]
    fn test_sha512_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"hello world\n").unwrap();

        let hash = compute_sha512(&path).unwrap();
        assert_eq!(hash.len(), 128); // SHA-512 hex digest is 128 chars
    }

    #[test]
    fn test_sha256_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"deterministic content").unwrap();

        let h1 = compute_sha256(&path).unwrap();
        let h2 = compute_sha256(&path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_verify_skip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"data").unwrap();

        assert!(verify_checksum(&path, "SKIP", ChecksumAlgo::Sha256).is_ok());
    }

    #[test]
    fn test_verify_sha256_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"test data").unwrap();

        let hash = compute_sha256(&path).unwrap();
        assert!(verify_checksum(&path, &hash, ChecksumAlgo::Sha256).is_ok());
    }

    #[test]
    fn test_verify_sha256_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"test data").unwrap();

        let bad = "0".repeat(64);
        let result = verify_checksum(&path, &bad, ChecksumAlgo::Sha256);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"test").unwrap();

        let hash = compute_sha256(&path).unwrap();
        let upper = hash.to_uppercase();
        assert!(verify_checksum(&path, &upper, ChecksumAlgo::Sha256).is_ok());
    }

    #[test]
    fn test_verify_sha512_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"sha512 test").unwrap();

        let hash = compute_sha512(&path).unwrap();
        assert!(verify_checksum(&path, &hash, ChecksumAlgo::Sha512).is_ok());
    }

    #[test]
    fn test_nonexistent_file() {
        let result = compute_sha256(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }
}
