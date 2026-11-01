use crate::SigmfError;
use sha2::Digest;
use std::io::Read;

pub fn hash_from_reader(reader: &mut dyn Read) -> Result<String, SigmfError> {
    let mut chunk = [0u8; 64 * 1024];

    let mut hasher = sha2::Sha512::new();
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break, // end of file
            Ok(n) => hasher.update(&chunk[..n]),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(SigmfError::Io(err)),
        }
    }
    let hash = hasher.finalize();
    let hash_str = hash.iter().map(|byte| format!("{:02x}", byte)).collect();
    Ok(hash_str)
}

pub fn hash_from_path(path: impl AsRef<std::path::Path>) -> Result<String, SigmfError> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let hash = hash_from_reader(&mut reader)?;
    Ok(hash)
}

pub fn hash_from_slice(data: &[u8]) -> Result<String, SigmfError> {
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let hash_str = hash.iter().map(|byte| format!("{:02x}", byte)).collect();
    Ok(hash_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

    struct InterruptedShortReader {
        cursor: Cursor<Vec<u8>>,
        interrupted: bool,
    }

    impl Read for InterruptedShortReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            let limit = buffer.len().min(3);
            self.cursor.read(&mut buffer[..limit])
        }
    }

    #[test]
    fn hashes_a_self_contained_temporary_fixture() {
        let path = std::env::temp_dir().join(format!(
            "sigmf-hash-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let fixture = b"sigmf hash fixture";
        std::fs::write(&path, fixture).unwrap();
        let hash = hash_from_path(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(hash, hash_from_slice(fixture).unwrap());
    }

    #[test]
    fn reader_hash_retries_interrupted_and_short_reads() {
        let fixture = b"short and interrupted";
        let mut reader = InterruptedShortReader {
            cursor: Cursor::new(fixture.to_vec()),
            interrupted: false,
        };
        assert_eq!(
            hash_from_reader(&mut reader).unwrap(),
            hash_from_slice(fixture).unwrap()
        );
    }
}
