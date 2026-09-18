use crate::minecraft::rules::{evaluate, RuleFeatures};
use crate::minecraft::version::ArgumentValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LaunchSubstitutions {
    pub auth_player_name: String,
    pub version_name: String,
    pub game_directory: String,
    pub assets_root: String,
    pub assets_index_name: String,
    pub auth_uuid: String,
    pub auth_uuid_undashed: String,
    pub auth_access_token: String,
    pub user_type: String,
    pub version_type: String,
    pub natives_directory: String,
    pub launcher_name: String,
    pub launcher_version: String,
    pub classpath: String,
    pub classpath_separator: String,
    pub primary_jar: String,
    pub resolution_width: Option<String>,
    pub resolution_height: Option<String>,
    pub quick_play_path: Option<String>,
    pub quick_play_singleplayer: Option<String>,
    pub quick_play_multiplayer: Option<String>,
    pub quick_play_realms: Option<String>,
}

impl LaunchSubstitutions {
    #[must_use]
    pub fn as_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("auth_player_name".into(), self.auth_player_name.clone());
        m.insert("version_name".into(), self.version_name.clone());
        m.insert("game_directory".into(), self.game_directory.clone());
        m.insert("assets_root".into(), self.assets_root.clone());
        m.insert("assets_index_name".into(), self.assets_index_name.clone());
        m.insert("auth_uuid".into(), self.auth_uuid.clone());
        m.insert("auth_access_token".into(), self.auth_access_token.clone());
        m.insert("user_type".into(), self.user_type.clone());
        m.insert("version_type".into(), self.version_type.clone());
        m.insert("natives_directory".into(), self.natives_directory.clone());
        m.insert("launcher_name".into(), self.launcher_name.clone());
        m.insert("launcher_version".into(), self.launcher_version.clone());
        m.insert("classpath".into(), self.classpath.clone());
        m.insert(
            "classpath_separator".into(),
            self.classpath_separator.clone(),
        );
        m.insert("primary_jar".into(), self.primary_jar.clone());
        m.insert("auth_uuid_undashed".into(), self.auth_uuid_undashed.clone());
        if let Some(v) = &self.resolution_width {
            m.insert("resolution_width".into(), v.clone());
        }
        if let Some(v) = &self.resolution_height {
            m.insert("resolution_height".into(), v.clone());
        }
        m
    }
}

#[must_use]
pub fn substitute(template: &str, map: &HashMap<String, String>) -> String {
    let mut out = template.to_string();
    for (k, v) in map {
        out = out.replace(&format!("${{{k}}}"), v);
    }
    out
}

#[must_use]
pub fn expand_args(
    args: &[ArgumentValue],
    features: &RuleFeatures,
    map: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for a in args {
        match a {
            ArgumentValue::Plain(s) => out.push(substitute(s, map)),
            ArgumentValue::Ruled { rules, value } => {
                if evaluate(Some(rules), features) {
                    for v in value.as_vec() {
                        out.push(substitute(&v, map));
                    }
                }
            }
        }
    }
    out
}

#[must_use]
pub fn expand_legacy_args(template: &str, map: &HashMap<String, String>) -> Vec<String> {
    let substituted = substitute(template, map);
    shell_split(&substituted)
}

fn shell_split(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            '\\' if in_quotes => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[must_use]
pub fn split_user_args(s: &str) -> Vec<String> {
    shell_split(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_map() -> HashMap<String, String> {
        [("auth_player_name".to_string(), "Steve".to_string())]
            .into_iter()
            .collect()
    }

    #[test]
    fn substitutes_placeholders() {
        let m = sample_map();
        assert_eq!(
            substitute("--username ${auth_player_name}", &m),
            "--username Steve"
        );
        assert_eq!(substitute("--x ${missing}", &m), "--x ${missing}");
    }

    #[test]
    fn legacy_split_respects_quotes() {
        let m = HashMap::new();
        let v = expand_legacy_args("--username Steve --msg \"hello world\"", &m);
        assert_eq!(v, vec!["--username", "Steve", "--msg", "hello world"]);
    }

    #[test]
    fn ruled_args_filtered() {
        use crate::minecraft::rules::{Rule, RuleAction};
        use crate::minecraft::version::ArgumentPayload;
        let args = vec![
            ArgumentValue::Plain("--always".into()),
            ArgumentValue::Ruled {
                rules: vec![Rule {
                    action: RuleAction::Allow,
                    os: None,
                    features: Some(
                        [("has_custom_resolution".to_string(), true)]
                            .into_iter()
                            .collect(),
                    ),
                }],
                value: ArgumentPayload::Single("--res".into()),
            },
        ];
        let m = HashMap::new();
        let out = expand_args(&args, &RuleFeatures::default(), &m);
        assert_eq!(out, vec!["--always"]);
        let f = RuleFeatures {
            has_custom_resolution: true,
            ..Default::default()
        };
        let out2 = expand_args(&args, &f, &m);
        assert_eq!(out2, vec!["--always", "--res"]);
    }
}
