use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JavaRuntime {
    pub path: PathBuf,
    pub major: u32,
    pub version_string: String,
    pub source: String,
}

impl JavaRuntime {
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("Java {} ({})", self.major, self.source)
    }
}

impl std::fmt::Display for JavaRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Java {} - {}", self.major, self.path.display())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaMode {
    Automatic,
    SystemDefault,
    Custom(PathBuf),
}

impl JavaMode {
    #[must_use]
    pub fn from_config(mode: &str, custom_path: &str) -> Self {
        Self::from_str_fallback(mode, custom_path)
    }
    #[must_use]
    pub fn from_str_fallback(mode: &str, custom_path: &str) -> Self {
        match mode {
            "system" => Self::SystemDefault,
            "custom" if !custom_path.trim().is_empty() => {
                Self::Custom(PathBuf::from(custom_path.trim()))
            }
            _ => Self::Automatic,
        }
    }
}

#[must_use]
pub fn select_runtime(
    runtimes: &[JavaRuntime],
    required_major: Option<u32>,
    mode: &JavaMode,
) -> Option<JavaRuntime> {
    match mode {
        JavaMode::Custom(p) => {
            if p.exists() {
                if let Some(hit) = runtimes.iter().find(|r| r.path == *p) {
                    return Some(hit.clone());
                }
                return Some(JavaRuntime {
                    path: p.clone(),
                    major: required_major.unwrap_or(17),
                    version_string: "custom".to_string(),
                    source: "custom".to_string(),
                });
            }
            return None;
        }
        JavaMode::SystemDefault => {
            return runtimes
                .iter()
                .find(|r| r.source == "PATH" || r.source == "JAVA_HOME")
                .or_else(|| runtimes.first())
                .cloned();
        }
        JavaMode::Automatic => {}
    }
    let mut sorted = runtimes.to_vec();
    sorted.sort_by_key(|r| r.major);
    if let Some(req) = required_major {
        if let Some(hit) = sorted.iter().find(|r| r.major == req) {
            return Some(hit.clone());
        }

        if let Some(hit) = sorted.iter().find(|r| r.major > req) {
            return Some(hit.clone());
        }
        return None;
    }

    sorted.into_iter().max_by_key(|r| r.major)
}

#[must_use]
pub fn required_major_for_version(minecraft_version: &str, meta_major: Option<u32>) -> Option<u32> {
    if let Some(m) = meta_major {
        return Some(m);
    }

    if let Ok(v) = semver_like(minecraft_version) {
        if v >= (1, 20, 5) {
            return Some(21);
        }
        if v >= (1, 18, 0) {
            return Some(17);
        }
        if v >= (1, 17, 0) {
            return Some(16);
        }
    }
    Some(8)
}

fn semver_like(s: &str) -> Result<(u64, u64, u64), ()> {
    let core = s.split('-').next().ok_or(())?;
    let mut it = core.split('.');
    let maj: u64 = it.next().ok_or(())?.parse().map_err(|_| ())?;
    let min: u64 = it.next().unwrap_or("0").parse().map_err(|_| ())?;
    let patch: u64 = it.next().unwrap_or("0").parse().map_err(|_| ())?;
    Ok((maj, min, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(major: u32, source: &str) -> JavaRuntime {
        JavaRuntime {
            path: PathBuf::from(format!("/java/{major}")),
            major,
            version_string: major.to_string(),
            source: source.to_string(),
        }
    }

    #[test]
    fn exact_match_wins() {
        let all = vec![rt(8, "x"), rt(17, "x"), rt(21, "x")];
        let sel = select_runtime(&all, Some(17), &JavaMode::Automatic).unwrap();
        assert_eq!(sel.major, 17);
    }

    #[test]
    fn higher_satisfies_when_no_exact() {
        let all = vec![rt(21, "x")];
        let sel = select_runtime(&all, Some(17), &JavaMode::Automatic).unwrap();
        assert_eq!(sel.major, 21);
    }

    #[test]
    fn lower_does_not_satisfy() {
        let all = vec![rt(8, "x")];
        assert!(select_runtime(&all, Some(17), &JavaMode::Automatic).is_none());
    }

    #[test]
    fn required_major_table() {
        assert_eq!(required_major_for_version("1.21", None), Some(21));
        assert_eq!(required_major_for_version("1.19.4", None), Some(17));
        assert_eq!(required_major_for_version("1.12.2", None), Some(8));
        assert_eq!(required_major_for_version("1.21", Some(21)), Some(21));
    }
}
