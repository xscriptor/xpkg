//! `.MTREE` generator.
//!
//! Produces a file integrity manifest listing every file, directory, and
//! symlink in the package with their SHA-256 checksums, permissions, sizes,
//! ownership, and modification times.
//!
//! Format: one entry per line, fields separated by spaces.
//! ```text
//! #mtree
//! ./usr type=dir mode=0755 uid=0 gid=0
//! ./usr/bin type=dir mode=0755 uid=0 gid=0
//! ./usr/bin/hello type=file mode=0755 size=12345 sha256digest=abc... uid=0 gid=0
//! ./usr/lib/libfoo.so type=link link=/usr/lib/libfoo.so.1 uid=0 gid=0
//! ```

use std::fmt::Write;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{XpkgError, XpkgResult};

/// Generate the `.MTREE` file content from the populated PKGDIR.
///
/// All uid/gid values are written as 0 (root) — the actual ownership
/// correction happens via tar header rewriting in the archive builder.
pub fn generate_mtree(pkgdir: &Path) -> XpkgResult<String> {
    let mut out = String::new();
    out.push_str("#mtree\n");

    let entries = collect_entries(pkgdir, pkgdir)?;

    // Sort for deterministic output.
    let mut entries = entries;
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in &entries {
        write_entry(&mut out, entry);
    }

    Ok(out)
}

/// A single file system entry in the manifest.
#[derive(Debug)]
struct MtreeEntry {
    /// Relative path from PKGDIR root, prefixed with `./`.
    path: String,
    /// Entry type.
    kind: EntryKind,
}

#[derive(Debug)]
enum EntryKind {
    Dir {
        mode: u32,
    },
    File {
        mode: u32,
        size: u64,
        sha256: String,
    },
    Link {
        target: String,
    },
}

/// Recursively collect all file system entries under `root`.
fn collect_entries(root: &Path, current: &Path) -> XpkgResult<Vec<MtreeEntry>> {
    let mut entries = Vec::new();

    let mut dir_entries: Vec<_> = std::fs::read_dir(current)
        .map_err(|e| {
            XpkgError::Archive(format!(
                "failed to read directory {}: {e}",
                current.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            XpkgError::Archive(format!(
                "failed to iterate directory {}: {e}",
                current.display()
            ))
        })?;

    // Sort entries for deterministic ordering.
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries {
        let path = entry.path();
        let rel = format!(
            "./{}",
            path.strip_prefix(root).unwrap_or(&path).to_string_lossy()
        );

        let symlink_meta = std::fs::symlink_metadata(&path).map_err(|e| {
            XpkgError::Archive(format!(
                "failed to read metadata for {}: {e}",
                path.display()
            ))
        })?;

        if symlink_meta.file_type().is_symlink() {
            let target = std::fs::read_link(&path)
                .map_err(|e| {
                    XpkgError::Archive(format!("failed to read symlink {}: {e}", path.display()))
                })?
                .to_string_lossy()
                .into_owned();

            entries.push(MtreeEntry {
                path: rel,
                kind: EntryKind::Link { target },
            });
        } else if symlink_meta.is_dir() {
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                symlink_meta.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = 0o755;

            entries.push(MtreeEntry {
                path: rel,
                kind: EntryKind::Dir { mode },
            });

            // Recurse into subdirectory.
            entries.extend(collect_entries(root, &path)?);
        } else {
            #[cfg(unix)]
            let mode = {
                use std::os::unix::fs::PermissionsExt;
                symlink_meta.permissions().mode() & 0o7777
            };
            #[cfg(not(unix))]
            let mode = 0o644;

            let size = symlink_meta.len();
            let sha256 = hash_file(&path)?;

            entries.push(MtreeEntry {
                path: rel,
                kind: EntryKind::File { mode, size, sha256 },
            });
        }
    }

    Ok(entries)
}

/// Compute SHA-256 hash of a file.
fn hash_file(path: &Path) -> XpkgResult<String> {
    let data = std::fs::read(path)
        .map_err(|e| XpkgError::Archive(format!("failed to read {}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Write a single mtree entry to the output string.
fn write_entry(out: &mut String, entry: &MtreeEntry) {
    match &entry.kind {
        EntryKind::Dir { mode } => {
            let _ = writeln!(out, "{} type=dir mode={:04o} uid=0 gid=0", entry.path, mode);
        }
        EntryKind::File { mode, size, sha256 } => {
            let _ = writeln!(
                out,
                "{} type=file mode={:04o} size={size} sha256digest={sha256} uid=0 gid=0",
                entry.path, mode
            );
        }
        EntryKind::Link { target } => {
            let _ = writeln!(out, "{} type=link link={target} uid=0 gid=0", entry.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mtree_header() {
        let tmp = tempfile::tempdir().unwrap();
        let mtree = generate_mtree(tmp.path()).unwrap();
        assert!(mtree.starts_with("#mtree\n"));
    }

    #[test]
    fn test_mtree_includes_files() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/hello"), "#!/bin/sh\necho hello").unwrap();

        let mtree = generate_mtree(pkgdir).unwrap();
        assert!(mtree.contains("./usr type=dir"));
        assert!(mtree.contains("./usr/bin type=dir"));
        assert!(mtree.contains("./usr/bin/hello type=file"));
        assert!(mtree.contains("sha256digest="));
        assert!(mtree.contains("uid=0 gid=0"));
    }

    #[test]
    fn test_mtree_includes_symlinks() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/lib")).unwrap();
        std::fs::write(pkgdir.join("usr/lib/libfoo.so.1"), "lib").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink("libfoo.so.1", pkgdir.join("usr/lib/libfoo.so")).unwrap();

        let mtree = generate_mtree(pkgdir).unwrap();

        #[cfg(unix)]
        assert!(mtree.contains("./usr/lib/libfoo.so type=link link=libfoo.so.1"));
    }

    #[test]
    fn test_mtree_deterministic_order() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::create_dir_all(pkgdir.join("usr/bin")).unwrap();
        std::fs::write(pkgdir.join("usr/bin/zzz"), "z").unwrap();
        std::fs::write(pkgdir.join("usr/bin/aaa"), "a").unwrap();

        let mtree1 = generate_mtree(pkgdir).unwrap();
        let mtree2 = generate_mtree(pkgdir).unwrap();
        assert_eq!(mtree1, mtree2);

        // aaa should come before zzz.
        let pos_aaa = mtree1.find("aaa").unwrap();
        let pos_zzz = mtree1.find("zzz").unwrap();
        assert!(pos_aaa < pos_zzz);
    }

    #[test]
    fn test_mtree_correct_sha256() {
        let tmp = tempfile::tempdir().unwrap();
        let pkgdir = tmp.path();
        std::fs::write(pkgdir.join("test.txt"), "hello").unwrap();

        let mtree = generate_mtree(pkgdir).unwrap();
        // SHA-256 of "hello" = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert!(mtree.contains(
            "sha256digest=2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        ));
    }

    #[test]
    fn test_mtree_empty_pkgdir() {
        let tmp = tempfile::tempdir().unwrap();
        let mtree = generate_mtree(tmp.path()).unwrap();
        assert_eq!(mtree, "#mtree\n");
    }
}
