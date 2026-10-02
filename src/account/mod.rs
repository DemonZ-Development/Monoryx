pub mod microsoft;
pub mod offline;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Account {
    Offline(offline::OfflineProfile),
    Microsoft(microsoft::MicrosoftProfile),
}

impl Account {
    #[must_use]
    pub fn username(&self) -> &str {
        match self {
            Self::Offline(p) => &p.username,
            Self::Microsoft(m) => &m.username,
        }
    }

    #[must_use]
    pub fn uuid(&self) -> uuid::Uuid {
        match self {
            Self::Offline(p) => p.uuid,
            Self::Microsoft(m) => m.uuid,
        }
    }

    #[must_use]
    pub const fn is_offline(&self) -> bool {
        matches!(self, Self::Offline(_))
    }

    #[must_use]
    pub fn access_token(&self) -> &str {
        match self {
            Self::Offline(_) => "0",
            Self::Microsoft(m) => &m.access_token,
        }
    }

    #[must_use]
    pub fn user_type(&self) -> &str {
        match self {
            Self::Offline(_) => "legacy",
            Self::Microsoft(_) => "msa",
        }
    }
}
