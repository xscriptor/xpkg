//! HTTP/HTTPS source downloader with retry support.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::XpkgError;

/// Options for downloading source files.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Maximum number of retry attempts.
    pub retries: u32,
    /// Connect timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Read timeout in seconds.
    pub read_timeout_secs: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            retries: 3,
            connect_timeout_secs: 30,
            read_timeout_secs: 300,
        }
    }
}

/// Extract a filename from a URL path component.
///
/// Strips query strings and fragments, returning `None` if the URL has
/// no identifiable filename.
pub fn filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next().unwrap_or(url);
    let path = path.split('#').next().unwrap_or(path);

    path.rsplit('/')
        .next()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.to_string())
}

/// Download a file from a URL to a destination path.
///
/// Retries up to `options.retries` times on transient failures.
pub fn download_file(
    url: &str,
    dest: &Path,
    options: &DownloadOptions,
) -> Result<PathBuf, XpkgError> {
    let mut last_error = None;

    for attempt in 1..=options.retries {
        tracing::info!(url, attempt, max = options.retries, "downloading source");

        match try_download(url, dest, options) {
            Ok(path) => return Ok(path),
            Err(e) => {
                tracing::warn!(url, attempt, error = %e, "download attempt failed");
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        XpkgError::SourceDownload(format!(
            "failed to download {url} after {} attempts",
            options.retries
        ))
    }))
}

/// Execute a single download attempt.
fn try_download(url: &str, dest: &Path, opts: &DownloadOptions) -> Result<PathBuf, XpkgError> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(opts.connect_timeout_secs))
        .timeout_read(std::time::Duration::from_secs(opts.read_timeout_secs))
        .build();

    let response = agent
        .get(url)
        .call()
        .map_err(|e| XpkgError::SourceDownload(format!("{url}: {e}")))?;

    let content_length: Option<u64> = response
        .header("content-length")
        .and_then(|v| v.parse().ok());

    if let Some(len) = content_length {
        tracing::debug!(url, bytes = len, "content-length");
    }

    // Ensure parent directory exists.
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            XpkgError::Io(io::Error::new(
                e.kind(),
                format!("failed to create directory {}: {e}", parent.display()),
            ))
        })?;
    }

    let mut file = File::create(dest).map_err(|e| {
        XpkgError::Io(io::Error::new(
            e.kind(),
            format!("failed to create {}: {e}", dest.display()),
        ))
    })?;

    let mut reader = response.into_reader();
    let mut buf = [0u8; 65536];
    let mut downloaded: u64 = 0;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| XpkgError::SourceDownload(format!("{url}: read error: {e}")))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
    }

    file.flush()?;

    tracing::info!(
        url,
        bytes = downloaded,
        dest = %dest.display(),
        "download complete"
    );

    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_from_url_simple() {
        assert_eq!(
            filename_from_url("https://example.com/releases/foo-1.0.tar.gz"),
            Some("foo-1.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_filename_from_url_query_string() {
        assert_eq!(
            filename_from_url("https://example.com/foo-1.0.tar.gz?token=abc"),
            Some("foo-1.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_filename_from_url_fragment() {
        assert_eq!(
            filename_from_url("https://example.com/foo-1.0.tar.gz#sha256=abc"),
            Some("foo-1.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_filename_from_url_no_extension() {
        assert_eq!(filename_from_url("https://example.com/download"), None);
    }

    #[test]
    fn test_filename_from_url_trailing_slash() {
        assert_eq!(filename_from_url("https://example.com/"), None);
    }

    #[test]
    fn test_download_options_default() {
        let opts = DownloadOptions::default();
        assert_eq!(opts.retries, 3);
        assert_eq!(opts.connect_timeout_secs, 30);
        assert_eq!(opts.read_timeout_secs, 300);
    }
}
