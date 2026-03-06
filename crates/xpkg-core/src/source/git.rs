//! Git source support — clone and checkout via system `git`.

use std::path::Path;
use std::process::Command;

use crate::XpkgError;

/// Check if a URL looks like a Git source.
///
/// Recognized patterns:
/// - `git://...`
/// - `git+https://...`
/// - `git+http://...`
/// - URLs ending in `.git`
pub fn is_git_url(url: &str) -> bool {
    url.starts_with("git://")
        || url.starts_with("git+https://")
        || url.starts_with("git+http://")
        || url.ends_with(".git")
}

/// Normalize a git URL by stripping the `git+` prefix.
fn normalize_url(url: &str) -> &str {
    url.strip_prefix("git+").unwrap_or(url)
}

/// Clone a git repository to a destination directory.
///
/// If `reference` is provided (tag, branch, or commit), it is passed to
/// `--branch` and the clone is performed with `--depth 1` for efficiency.
pub fn git_clone(url: &str, dest: &Path, reference: Option<&str>) -> Result<(), XpkgError> {
    let normalized = normalize_url(url);

    tracing::info!(
        url = normalized,
        dest = %dest.display(),
        "cloning git repository"
    );

    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if let Some(refspec) = reference {
        cmd.arg("--depth").arg("1").arg("--branch").arg(refspec);
    }

    cmd.arg(normalized).arg(dest);

    let output = cmd
        .output()
        .map_err(|e| XpkgError::SourceDownload(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(XpkgError::SourceDownload(format!(
            "git clone failed: {stderr}"
        )));
    }

    tracing::info!(dest = %dest.display(), "git clone complete");
    Ok(())
}

/// Check out a specific reference (tag, branch, or commit) in an existing
/// repository.
pub fn git_checkout(repo: &Path, reference: &str) -> Result<(), XpkgError> {
    tracing::info!(
        repo = %repo.display(),
        reference,
        "checking out git reference"
    );

    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("checkout")
        .arg(reference)
        .output()
        .map_err(|e| XpkgError::SourceDownload(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(XpkgError::SourceDownload(format!(
            "git checkout failed: {stderr}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url_git_protocol() {
        assert!(is_git_url("git://github.com/user/repo.git"));
    }

    #[test]
    fn test_is_git_url_git_plus_https() {
        assert!(is_git_url("git+https://github.com/user/repo.git"));
    }

    #[test]
    fn test_is_git_url_git_plus_http() {
        assert!(is_git_url("git+http://github.com/user/repo"));
    }

    #[test]
    fn test_is_git_url_dot_git_suffix() {
        assert!(is_git_url("https://github.com/user/repo.git"));
    }

    #[test]
    fn test_not_git_url() {
        assert!(!is_git_url("https://example.com/foo-1.0.tar.gz"));
        assert!(!is_git_url("ftp://mirror.example.com/releases/bar.tar.xz"));
    }

    #[test]
    fn test_normalize_url_strips_prefix() {
        assert_eq!(
            normalize_url("git+https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn test_normalize_url_passthrough() {
        assert_eq!(
            normalize_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }
}
