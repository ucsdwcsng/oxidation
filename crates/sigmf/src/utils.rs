use crate::SmfError;
use sha2::Digest;
use std::io::Read;

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
