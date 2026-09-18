use crate::error::{MonoryxError, Result};
use sha1::Digest as _;
use std::path::Path;

pub fn sha1_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut h = sha1::Sha1::new();
    h.update(&bytes);
    Ok(hex::encode(h.finalize()))
}

pub fn sha512_file(path: &Path) -> Result<String> {
    use sha2::Digest as _;
    let bytes = std::fs::read(path)?;
    let mut h = sha2::Sha512::new();
    h.update(&bytes);
    Ok(hex::encode(h.finalize()))
}

#[must_use]
pub fn sha1_bytes(bytes: &[u8]) -> String {
    let mut h = sha1::Sha1::new();
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
