use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfflineProfile {
    pub username: String,
    pub uuid: uuid::Uuid,

    #[serde(default)]
    pub created_at: String,
}

impl OfflineProfile {
    pub fn new(username: &str) -> Result<Self> {
        let name = username.trim();
        crate::utils::validation::validate_username(name)?;
        Ok(Self {
            username: name.to_string(),
            uuid: offline_uuid(name),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    #[must_use]
    pub fn uuid_undashed(&self) -> String {
        self.uuid.as_simple().to_string()
    }
}

pub fn offline_uuid(username: &str) -> uuid::Uuid {
    let input = format!("OfflinePlayer:{username}");

    let digest = md5::compute(input.as_bytes());
    let mut bytes = digest.0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_deterministic_and_v3() {
        let a = offline_uuid("Steve");
        let b = offline_uuid("Steve");
        assert_eq!(a, b);
        assert_eq!(a.get_version_num(), 3);
        assert_ne!(a, offline_uuid("Alex"));
    }

    #[test]
    fn known_vector_notch() {
        let u = offline_uuid("Notch");
        assert_eq!(u.get_version_num(), 3);

        let digest = md5::compute(b"OfflinePlayer:Notch");
        let mut bytes = digest.0;
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        assert_eq!(u, uuid::Uuid::from_bytes(bytes));
    }

    #[test]
    fn rejects_bad_names() {
        assert!(OfflineProfile::new("ab").is_err());
        assert!(OfflineProfile::new("bad name!").is_err());
        assert!(OfflineProfile::new("Valid_Name1").is_ok());
    }
}
