use crate::error::{MonoryxError, Result};

pub fn validate_username(name: &str) -> Result<()> {
    let len = name.chars().count();
    if !(3..=16).contains(&len) {
        return Err(MonoryxError::InvalidUsername(
            "Username must be 3-16 characters.".to_string(),
        ));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(MonoryxError::InvalidUsername(
            "Only letters, numbers and underscore are allowed.".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_instance_name(name: &str) -> Result<()> {
    let t = name.trim();
    if t.is_empty() || t.len() > 48 {
        return Err(MonoryxError::Instance(
            "Instance name must be 1-48 characters.".to_string(),
        ));
    }
    if t.contains(['/', '\\', '\0']) {
        return Err(MonoryxError::Instance(
            "Instance name must not contain path separators.".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_ok() {
        assert!(validate_username("Steve_123").is_ok());
    }

    #[test]
    fn username_bad() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("this_name_is_way_too_long").is_err());
        assert!(validate_username("bad-name!").is_err());
        assert!(validate_username("with space").is_err());
    }
}
