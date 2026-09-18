use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Relevance,
    Downloads,
    Updated,
    Newest,
}

impl SortOrder {
    #[must_use]
    pub const fn api_value(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Downloads => "downloads",
            Self::Updated => "updated",
            Self::Newest => "newest",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Relevance => "Relevance",
            Self::Downloads => "Downloads",
            Self::Updated => "Recently updated",
            Self::Newest => "Newest",
        }
    }

    #[must_use]
    pub const fn all() -> [Self; 4] {
        [
            Self::Relevance,
            Self::Downloads,
            Self::Updated,
            Self::Newest,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub sort: SortOrder,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            query: String::new(),
            project_type: "mod".to_string(),
            game_version: String::new(),
            loader: String::new(),
            sort: SortOrder::Relevance,
            limit: 24,
            offset: 0,
        }
    }
}
