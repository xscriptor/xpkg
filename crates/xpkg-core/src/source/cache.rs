//! Source file caching to avoid redundant downloads.
//!
//! Files are stored using a truncated SHA-256 hash of the URL as the
//! cache key, preserving the original file extension.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::XpkgError;

/// Source cache backed by a directory on disk.
#[derive(Debug, Clone)]
pub struct SourceCache {
    cache_dir: PathBuf,
}

impl SourceCache {
    /// Create a new source cache at the given directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Return the default cache directory (`$XDG_CACHE_HOME/xpkg/sources`
    /// or `~/.cache/xpkg/sources`).
    pub fn default_dir() -> PathBuf {
        std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".cache")
            })
            .join("xpkg")
            .join("sources")
    }

    /// Look up a cached copy of a source URL.
    ///
    /// Returns `Some(path)` if the file exists in the cache.
    pub fn get(&self, url: &str) -> Option<PathBuf> {
        let path = self.cache_path(url);
        if path.exists() {
            tracing::debug!(url, path = %path.display(), "cache hit");
            Some(path)
        } else {
            tracing::debug!(url, "cache miss");
            None
        }
    }

    /// Store a downloaded file in the cache.
    ///
    /// Copies the file into the cache directory using the URL-based key.
    pub fn store(&self, url: &str, source: &Path) -> Result<PathBuf, XpkgError> {
        let dest = self.cache_path(url);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                XpkgError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to create cache dir {}: {e}", parent.display()),
                ))
            })?;
        }

        fs::copy(source, &dest).map_err(|e| {
            XpkgError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "failed to cache {} → {}: {e}",
                    source.display(),
                    dest.display()
                ),
            ))
        })?;

        tracing::debug!(url, path = %dest.display(), "stored in cache");
        Ok(dest)
    }

    /// Compute the cache file path for a URL.
    fn cache_path(&self, url: &str) -> PathBuf {
        let key = cache_key(url);
        let ext = url_extension(url).unwrap_or_default();
        if ext.is_empty() {
            self.cache_dir.join(key)
        } else {
            self.cache_dir.join(format!("{key}.{ext}"))
        }
    }
}

/// Compute a cache key from a URL (truncated SHA-256 hex, 16 chars).
fn cache_key(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

/// Extract the file extension from a URL path, handling compound
/// extensions like `.tar.gz`.
fn url_extension(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let name = path.rsplit('/').next()?;

    for compound in &[".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if name.ends_with(compound) {
            return Some(compound[1..].to_string());
        }
    }

    // Simple extension.
    name.rsplit_once('.').map(|(_, ext)| ext.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_deterministic() {
        let k1 = cache_key("https://example.com/foo-1.0.tar.gz");
        let k2 = cache_key("https://example.com/foo-1.0.tar.gz");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 16);
    }

    #[test]
    fn test_cache_key_different_urls() {
        let k1 = cache_key("https://example.com/foo.tar.gz");
        let k2 = cache_key("https://example.com/bar.tar.gz");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_url_extension_tar_gz() {
        assert_eq!(
            url_extension("https://example.com/foo-1.0.tar.gz"),
            Some("tar.gz".to_string())
        );
    }

    #[test]
    fn test_url_extension_tar_xz() {
        assert_eq!(
            url_extension("https://example.com/foo-1.0.tar.xz"),
            Some("tar.xz".to_string())
        );
    }

    #[test]
    fn test_url_extension_zip() {
        assert_eq!(
            url_extension("https://example.com/foo.zip"),
            Some("zip".to_string())
        );
    }

    #[test]
    fn test_url_extension_with_query() {
        assert_eq!(
            url_extension("https://example.com/foo.tar.gz?token=abc"),
            Some("tar.gz".to_string())
        );
    }

    #[test]
    fn test_url_extension_none() {
        assert_eq!(url_extension("https://example.com/download"), None);
    }

    #[test]
    fn test_default_dir() {
        let dir = SourceCache::default_dir();
        assert!(dir.ends_with("xpkg/sources"));
    }

    #[test]
    fn test_cache_store_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let cache = SourceCache::new(dir.path().to_path_buf());
        let url = "https://example.com/foo-1.0.tar.gz";

        // Initially not cached.
        assert!(cache.get(url).is_none());

        // Create a fake source file.
        let src = dir.path().join("foo-1.0.tar.gz");
        std::fs::write(&src, b"fake archive data").unwrap();

        // Store in cache.
        let cached_path = cache.store(url, &src).unwrap();
        assert!(cached_path.exists());

        // Now should be a hit.
        let hit = cache.get(url);
        assert!(hit.is_some());
        assert_eq!(std::fs::read(hit.unwrap()).unwrap(), b"fake archive data");
    }
}
