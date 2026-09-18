use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<std::collections::HashMap<String, bool>>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleFeatures {
    pub is_demo_user: bool,
    pub has_custom_resolution: bool,
    pub has_quick_plays_support: bool,
    pub is_quick_play_singleplayer: bool,
    pub is_quick_play_multiplayer: bool,
    pub is_quick_play_realms: bool,
    pub has_content: bool,
}

impl Rule {
    fn os_matches(&self) -> bool {
        let Some(os) = &self.os else {
            return true;
        };
        if let Some(name) = &os.name {
            let current = crate::utils::system::mojang_os();
            if name != current {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            let current = crate::utils::system::mojang_arch();
            if arch != current {
                let na = match arch.as_str() {
                    "x86_64" | "amd64" => "x64",
                    o => o,
                };
                let nc = match current {
                    "x86_64" | "amd64" => "x64",
                    o => o,
                };
                if na != nc {
                    return false;
                }
            }
        }
        if let Some(pattern) = &os.version {
            if !os_version_matches(pattern) {
                return false;
            }
        }
        true
    }

    fn features_match(&self, features: &RuleFeatures) -> bool {
        let Some(map) = &self.features else {
            return true;
        };
        for (k, v) in map {
            let actual = match k.as_str() {
                "is_demo_user" => features.is_demo_user,
                "has_custom_resolution" => features.has_custom_resolution,
                "has_quick_plays_support" => features.has_quick_plays_support,
                "is_quick_play_singleplayer" => features.is_quick_play_singleplayer,
                "is_quick_play_multiplayer" => features.is_quick_play_multiplayer,
                "is_quick_play_realms" => features.is_quick_play_realms,
                "has_content" => features.has_content,
                _ => continue,
            };
            if actual != *v {
                return false;
            }
        }
        true
    }
}

#[must_use]
pub fn evaluate(rules: Option<&[Rule]>, features: &RuleFeatures) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule.os_matches() && rule.features_match(features) {
            allowed = matches!(rule.action, RuleAction::Allow);
        }
    }
    allowed
}

fn os_version_matches(pattern: &str) -> bool {
    let current = std::env::consts::OS;

    let os_version = current_os_version();

    if let Some(prefix) = pattern.strip_prefix('^') {
        let unescaped = prefix.replace("\\.", ".").replace("\\\\", "\\");

        let clean = unescaped
            .trim_end_matches('$')
            .trim_end_matches(".*")
            .trim_end_matches('.');
        if clean.is_empty() {
            return true;
        }
        return os_version.starts_with(clean) || current.starts_with(clean);
    }
    os_version.contains(pattern) || current.contains(pattern)
}

#[cfg(target_os = "windows")]
fn current_os_version() -> String {
    "10.0".to_string()
}

#[cfg(target_os = "linux")]
fn current_os_version() -> String {
    "linux".to_string()
}

#[cfg(target_os = "macos")]
fn current_os_version() -> String {
    "macos".to_string()
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn current_os_version() -> String {
    std::env::consts::OS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow_os(name: &str) -> Rule {
        Rule {
            action: RuleAction::Allow,
            os: Some(OsRule {
                name: Some(name.to_string()),
                version: None,
                arch: None,
            }),
            features: None,
        }
    }

    #[test]
    fn empty_rules_allowed() {
        assert!(evaluate(None, &RuleFeatures::default()));
        assert!(evaluate(Some(&[]), &RuleFeatures::default()));
    }

    #[test]
    fn os_allow_disallow_ordering() {
        let rules = vec![
            Rule {
                action: RuleAction::Allow,
                os: None,
                features: None,
            },
            Rule {
                action: RuleAction::Disallow,
                os: Some(OsRule {
                    name: Some(crate::utils::system::mojang_os().to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            },
        ];
        assert!(!evaluate(Some(&rules), &RuleFeatures::default()));
    }

    #[test]
    fn non_matching_os_ignored() {
        let other = if crate::utils::system::mojang_os() == "windows" {
            "linux"
        } else {
            "windows"
        };
        let rules = vec![allow_os(other)];
        assert!(!evaluate(Some(&rules), &RuleFeatures::default()));
    }

    #[test]
    fn maven_style_feature_gate() {
        let rules = vec![Rule {
            action: RuleAction::Allow,
            os: None,
            features: Some(
                [("has_custom_resolution".to_string(), true)]
                    .into_iter()
                    .collect(),
            ),
        }];
        assert!(!evaluate(Some(&rules), &RuleFeatures::default()));
        let f = RuleFeatures {
            has_custom_resolution: true,
            ..Default::default()
        };
        assert!(evaluate(Some(&rules), &f));
    }
}
