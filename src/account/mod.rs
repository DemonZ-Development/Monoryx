pub mod offline;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Account {
    Offline(offline::OfflineProfile),
}

impl Account {
    #[must_use]
    pub fn username(&self) -> &str {
        match self {
            Self::Offline(p) => &p.username,
        }
    }

    #[must_use]
    pub fn uuid(&self) -> uuid::Uuid {
        match self {
            Self::Offline(p) => p.uuid,
        }
    }

    #[must_use]
    pub const fn is_offline(&self) -> bool {
        true
    }
}
