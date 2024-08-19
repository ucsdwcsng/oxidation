use crate::{SmfError, MAX_META_SIZE};
use sha2::Digest;
use std::fs::File;
use std::io::Read;
use tar::Archive;

pub fn get_hash_from(path: &String) -> Result<String, SmfError> {
    let file = std::fs::File::open(&path)?;
    let mut reader = std::io::BufReader::new(file);
    let hash = hash_from_reader(&mut reader)?;
    Ok(hash)
}

pub fn hash_from_reader(reader: &mut dyn Read) -> Result<String, SmfError> {
    let mut chunk = [0u8; 1024]; // adjust chunk size as needed

    let mut hasher = sha2::Sha512::new();
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break, // end of file
            Ok(n) => hasher.update(&chunk[..n]),
            Err(err) => return Err(SmfError::Io(err)),
        }
    }
    let hash = hasher.finalize();
    let hash_str = format!("{:x}", hash);
    Ok(hash_str)
}

pub fn hash_from_slice(data: &[u8]) -> Result<String, SmfError> {
    let mut hasher = sha2::Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let hash_str = format!("{:x}", hash);
    Ok(hash_str)
}

pub fn get_root_dir() -> String {
    let mut path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    path.push_str("/../..");
    let canonicalized = std::fs::canonicalize(&path).unwrap();
    canonicalized.to_str().unwrap().to_string()
}

pub fn get_bytes_in_archive_from_name_ext(
    archive_path: &str,
    name: &str,
    ext: &str,
) -> Result<Vec<u8>, SmfError> {
    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        let file_path = file.path()?.to_path_buf();

        if let (Some(file_name), Some(file_ext)) = (
            file_path.file_stem().and_then(|s| s.to_str()),
            file_path.extension().and_then(|s| s.to_str()),
        ) {
            if file_name == name && file_ext == ext {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                return Ok(buffer);
            }
        }
    }

    Err(SmfError::NotFound("File not found".to_string()))
}

pub fn get_bytes_in_archive_from_hash(archive_path: &str, hash: &str) -> Result<Vec<u8>, SmfError> {
    let file = File::open(archive_path)?;
    let mut archive = Archive::new(file);

    for file in archive.entries()? {
        let mut file = file?;
        if file.size() > MAX_META_SIZE as u64 {
            continue;
        }
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let hash_str = hash_from_slice(&buffer[..])?;
        println!("hash_str: {}", hash_str);
        if hash_str == hash {
            return Ok(buffer);
        }
    }

    Err(SmfError::NotFound("File not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hash_from() {
        // Test the hash of the LICENSE file since it is a known file
        let root_dir = get_root_dir();
        let path = format!("{}/LICENSE", root_dir);
        let hash = get_hash_from(&path).unwrap();
        assert_eq!(hash, "b3c70582ebddd4c287211f2545f7c1305e1d7693df24218b32002774ed24e4a7df9989f6ad0d2ddb69b76f8390fdc7783a847bd81b94762d17e790cbe0835d6d");
    }
}
