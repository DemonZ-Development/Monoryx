use crate::error::{MonoryxError, Result};
use sha1::Digest as _;
use std::io::{BufReader, Read};
use std::path::Path;

fn stream_file(path: &Path, mut update: impl FnMut(&[u8])) -> Result<()> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        update(&buffer[..count]);
    }
    Ok(())
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut h = sha1::Sha1::new();
    stream_file(path, |bytes| h.update(bytes))?;
    Ok(hex::encode(h.finalize()))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use sha2::Digest as _;
    let mut h = sha2::Sha256::new();
    stream_file(path, |bytes| h.update(bytes))?;
    Ok(hex::encode(h.finalize()))
}

pub fn sha512_file(path: &Path) -> Result<String> {
    use sha2::Digest as _;
    let mut h = sha2::Sha512::new();
    stream_file(path, |bytes| h.update(bytes))?;
    Ok(hex::encode(h.finalize()))
}

#[must_use]
pub fn sha1_bytes(bytes: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[must_use]
pub fn sha512_bytes(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut h = sha2::Sha512::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

pub fn verify_file(path: &Path, algo: &str, expected: &str) -> Result<()> {
    let expected = expected.to_lowercase();
    let actual = match algo {
        "sha1" => sha1_file(path)?,
        "sha256" => sha256_file(path)?,
        "sha512" => sha512_file(path)?,
        other => {
            return Err(MonoryxError::Download(format!(
                "unsupported hash algorithm: {other}"
            )));
        }
    };
    if actual.to_lowercase() != expected {
        return Err(MonoryxError::HashMismatch {
            file: path.display().to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

pub fn verify_bytes(bytes: &[u8], algo: &str, expected: &str) -> Result<()> {
    let expected = expected.to_lowercase();
    let actual = match algo {
        "sha1" => sha1_bytes(bytes),
        "sha256" => sha256_bytes(bytes),
        "sha512" => sha512_bytes(bytes),
        other => {
            return Err(MonoryxError::Download(format!(
                "unsupported hash algorithm: {other}"
            )));
        }
    };
    if actual.to_lowercase() != expected {
        return Err(MonoryxError::HashMismatch {
            file: "<memory>".to_string(),
            expected,
            actual,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_known_vector() {
        assert_eq!(
            sha1_bytes(b"hello"),
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_bytes(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn verify_bytes_ok_and_mismatch() {
        let h = sha1_bytes(b"abc");
        assert!(verify_bytes(b"abc", "sha1", &h).is_ok());
        let err = verify_bytes(b"abd", "sha1", &h).unwrap_err();
        assert!(matches!(err, MonoryxError::HashMismatch { .. }));
    }

    #[test]
    fn unsupported_algo_errors() {
        assert!(verify_bytes(b"x", "md9", "00").is_err());
    }
}
