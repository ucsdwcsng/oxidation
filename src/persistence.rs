use crate::SigmfError;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn write_atomic(
    destination: &Path,
    bytes: &[u8],
    replace: bool,
) -> Result<(), SigmfError> {
    let parent = parent_directory(destination);
    let (temporary_path, mut file) = create_sibling_temp(destination)?;

    let result = (|| {
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        if replace {
            fs::rename(&temporary_path, destination)?;
        } else {
            publish_new(&temporary_path, destination)?;
        }
        if replace {
            sync_directory(parent)?;
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

pub(crate) fn create_sibling_temp(destination: &Path) -> Result<(PathBuf, File), SigmfError> {
    let parent = parent_directory(destination);
    let file_name = destination
        .file_name()
        .ok_or_else(|| SigmfError::InvalidMetadata("destination must name a file".to_string()))?;
    for attempt in 0..16 {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| SigmfError::InvalidMetadata(error.to_string()))?
            .as_nanos();
        let mut temporary_name = std::ffi::OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".{}-{nonce}-{attempt}.tmp", std::process::id()));
        let path = parent.join(temporary_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(SigmfError::Io(error)),
        }
    }
    Err(SigmfError::ResourceLimit(
        "could not allocate a unique sibling temporary file".to_string(),
    ))
}

pub(crate) fn publish_new(temporary_path: &Path, destination: &Path) -> Result<(), SigmfError> {
    let parent = parent_directory(destination);
    fs::hard_link(temporary_path, destination)?;
    fs::remove_file(temporary_path)?;
    sync_directory(parent)
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), SigmfError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), SigmfError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_file_paths_use_the_current_directory_as_parent() {
        assert_eq!(
            parent_directory(Path::new("metadata.sigmf-meta")),
            Path::new(".")
        );
        assert_eq!(
            parent_directory(Path::new("data/metadata.sigmf-meta")),
            Path::new("data")
        );
    }

    #[test]
    fn refuses_overwrite_and_replaces_atomically_when_requested() {
        let directory = std::env::temp_dir().join(format!(
            "sigmf-persistence-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("metadata.sigmf-meta");
        write_atomic(&target, b"first", false).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"first");
        assert!(write_atomic(&target, b"second", false).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"first");
        write_atomic(&target, b"second", true).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
        fs::remove_dir_all(directory).unwrap();
    }
}
