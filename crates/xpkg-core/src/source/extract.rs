//! Archive extraction — tar.gz, tar.xz, tar.bz2, tar.zst, and zip.

use std::fs::File;
use std::path::Path;

use crate::XpkgError;

/// Supported archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    TarXz,
    TarBz2,
    TarZst,
    Zip,
}

/// Detect the archive format from a file path extension.
pub fn detect_format(path: &Path) -> Option<ArchiveFormat> {
    let name = path.file_name()?.to_str()?;
    let lower = name.to_lowercase();

    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Some(ArchiveFormat::TarGz)
    } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        Some(ArchiveFormat::TarXz)
    } else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        Some(ArchiveFormat::TarBz2)
    } else if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        Some(ArchiveFormat::TarZst)
    } else if lower.ends_with(".zip") {
        Some(ArchiveFormat::Zip)
    } else {
        None
    }
}

/// Extract an archive into a destination directory.
///
/// The format is auto-detected from the file extension.
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let format = detect_format(archive).ok_or_else(|| {
        XpkgError::Archive(format!(
            "unrecognized archive format: {}",
            archive.display()
        ))
    })?;

    std::fs::create_dir_all(dest).map_err(|e| {
        XpkgError::Io(std::io::Error::new(
            e.kind(),
            format!("failed to create extraction dir {}: {e}", dest.display()),
        ))
    })?;

    match format {
        ArchiveFormat::TarGz => extract_tar_gz(archive, dest),
        ArchiveFormat::TarXz => extract_tar_xz(archive, dest),
        ArchiveFormat::TarBz2 => extract_tar_bz2(archive, dest),
        ArchiveFormat::TarZst => extract_tar_zst(archive, dest),
        ArchiveFormat::Zip => extract_zip(archive, dest),
    }
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let file = open_archive(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| archive_error(archive, &e))?;
    tracing::info!(archive = %archive.display(), "extracted tar.gz");
    Ok(())
}

fn extract_tar_xz(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let file = open_archive(archive)?;
    let decoder = xz2::read::XzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| archive_error(archive, &e))?;
    tracing::info!(archive = %archive.display(), "extracted tar.xz");
    Ok(())
}

fn extract_tar_bz2(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let file = open_archive(archive)?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| archive_error(archive, &e))?;
    tracing::info!(archive = %archive.display(), "extracted tar.bz2");
    Ok(())
}

fn extract_tar_zst(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let file = open_archive(archive)?;
    let decoder = zstd::stream::read::Decoder::new(file).map_err(|e| archive_error(archive, &e))?;
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| archive_error(archive, &e))?;
    tracing::info!(archive = %archive.display(), "extracted tar.zst");
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), XpkgError> {
    let file = open_archive(archive)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        XpkgError::Archive(format!("failed to open zip {}: {e}", archive.display()))
    })?;
    zip.extract(dest).map_err(|e| {
        XpkgError::Archive(format!("failed to extract zip {}: {e}", archive.display()))
    })?;
    tracing::info!(archive = %archive.display(), "extracted zip");
    Ok(())
}

/// Open an archive file for reading.
fn open_archive(path: &Path) -> Result<File, XpkgError> {
    File::open(path).map_err(|e| {
        XpkgError::Io(std::io::Error::new(
            e.kind(),
            format!("failed to open archive {}: {e}", path.display()),
        ))
    })
}

/// Create an archive error from an I/O error.
fn archive_error(path: &Path, e: &std::io::Error) -> XpkgError {
    XpkgError::Archive(format!("failed to extract {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_detect_tar_gz() {
        assert_eq!(
            detect_format(Path::new("foo-1.0.tar.gz")),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            detect_format(Path::new("foo.tgz")),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn test_detect_tar_xz() {
        assert_eq!(
            detect_format(Path::new("foo-1.0.tar.xz")),
            Some(ArchiveFormat::TarXz)
        );
        assert_eq!(
            detect_format(Path::new("foo.txz")),
            Some(ArchiveFormat::TarXz)
        );
    }

    #[test]
    fn test_detect_tar_bz2() {
        assert_eq!(
            detect_format(Path::new("foo.tar.bz2")),
            Some(ArchiveFormat::TarBz2)
        );
        assert_eq!(
            detect_format(Path::new("foo.tbz2")),
            Some(ArchiveFormat::TarBz2)
        );
    }

    #[test]
    fn test_detect_tar_zst() {
        assert_eq!(
            detect_format(Path::new("foo.tar.zst")),
            Some(ArchiveFormat::TarZst)
        );
        assert_eq!(
            detect_format(Path::new("foo.tzst")),
            Some(ArchiveFormat::TarZst)
        );
    }

    #[test]
    fn test_detect_zip() {
        assert_eq!(
            detect_format(Path::new("foo.zip")),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_format(Path::new("foo.txt")), None);
        assert_eq!(detect_format(Path::new("foo.tar")), None);
    }

    #[test]
    fn test_detect_case_insensitive() {
        assert_eq!(
            detect_format(Path::new("Foo.TAR.GZ")),
            Some(ArchiveFormat::TarGz)
        );
        assert_eq!(
            detect_format(Path::new("BAR.ZIP")),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn test_extract_tar_gz_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("test.tar.gz");

        // Create a tar.gz with a test file.
        let file = File::create(&archive_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut tar_builder = tar::Builder::new(enc);
        let data = b"hello from tar.gz\n";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder
            .append_data(&mut header, "test.txt", &data[..])
            .unwrap();
        tar_builder.into_inner().unwrap().finish().unwrap();

        // Extract.
        let out = dir.path().join("out");
        extract_archive(&archive_path, &out).unwrap();

        let content = std::fs::read_to_string(out.join("test.txt")).unwrap();
        assert_eq!(content, "hello from tar.gz\n");
    }

    #[test]
    fn test_extract_tar_zst_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("test.tar.zst");

        // Create a tar in memory, then compress with zstd.
        let mut tar_data = Vec::new();
        {
            let mut tar_builder = tar::Builder::new(&mut tar_data);
            let data = b"hello from tar.zst\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar_builder
                .append_data(&mut header, "zst_test.txt", &data[..])
                .unwrap();
            tar_builder.finish().unwrap();
        }

        let compressed = zstd::encode_all(tar_data.as_slice(), 3).unwrap();
        std::fs::write(&archive_path, compressed).unwrap();

        // Extract.
        let out = dir.path().join("out");
        extract_archive(&archive_path, &out).unwrap();

        let content = std::fs::read_to_string(out.join("zst_test.txt")).unwrap();
        assert_eq!(content, "hello from tar.zst\n");
    }

    #[test]
    fn test_extract_zip_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let archive_path = dir.path().join("test.zip");

        // Create a zip file.
        let file = File::create(&archive_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip_writer.start_file("zip_test.txt", options).unwrap();
        zip_writer.write_all(b"hello from zip\n").unwrap();
        zip_writer.finish().unwrap();

        // Extract.
        let out = dir.path().join("out");
        extract_archive(&archive_path, &out).unwrap();

        let content = std::fs::read_to_string(out.join("zip_test.txt")).unwrap();
        assert_eq!(content, "hello from zip\n");
    }

    #[test]
    fn test_extract_unrecognized_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, b"not an archive").unwrap();

        let result = extract_archive(&path, &dir.path().join("out"));
        assert!(result.is_err());
    }
}
