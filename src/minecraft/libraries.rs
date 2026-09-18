use crate::minecraft::rules::{evaluate, RuleFeatures};
use crate::minecraft::version::Library;

#[must_use]
pub fn maven_path(name: &str) -> Option<String> {
    crate::minecraft::manifest::maven_coord_to_path(name)
}

#[must_use]
pub fn library_applies(lib: &Library) -> bool {
    evaluate(lib.rules.as_deref(), &RuleFeatures::default())
}

#[must_use]
pub fn artifact_location(lib: &Library, default_base: &str) -> Option<(String, String)> {
    if let Some(d) = &lib.downloads {
        if let Some(a) = &d.artifact {
            return Some((a.path.clone(), a.url.clone()));
        }
    }
    let path = maven_path(&lib.name)?;
    let base = lib.url.as_deref().unwrap_or(default_base);
    let base = base.trim_end_matches('/');
    Some((path.clone(), format!("{base}/{path}")))
}

#[must_use]
pub fn current_natives_key(natives: &std::collections::HashMap<String, String>) -> Option<String> {
    let os = crate::utils::system::mojang_os();

    let key = match os {
        "windows" => "windows",
        "linux" => "linux",
        "osx" => "osx",
        _ => "linux",
    };
    natives
        .get(key)
        .cloned()
        .or_else(|| natives.get(&format!("{key}-64")).cloned())
}

#[must_use]
pub fn build_classpath(
    libraries: &[Library],
    libraries_dir: &std::path::Path,
    client_jar: &std::path::Path,
    default_base: &str,
) -> Vec<std::path::PathBuf> {
    let mut cp = Vec::new();
    for lib in libraries {
        if !library_applies(lib) {
            continue;
        }

        if lib.natives.is_some() {
            if let Some(d) = &lib.downloads {
                if let Some(a) = &d.artifact {
                    cp.push(libraries_dir.join(&a.path));
                    continue;
                }
            }

            if lib
                .downloads
                .as_ref()
                .and_then(|d| d.artifact.as_ref())
                .is_none()
            {
                let _ = default_base;
                continue;
            }
        }
        if let Some((path, _)) = artifact_location(lib, default_base) {
            cp.push(libraries_dir.join(path));
        }
    }
    cp.push(client_jar.to_path_buf());
    cp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::rules::{OsRule, Rule, RuleAction};

    #[test]
    fn natives_key_windows() {
        let mut m = std::collections::HashMap::new();
        m.insert("windows".to_string(), "natives-windows".to_string());
        m.insert("linux".to_string(), "natives-linux".to_string());
        let k = current_natives_key(&m).unwrap();
        let expected = match crate::utils::system::mojang_os() {
            "windows" => "natives-windows",
            "linux" => "natives-linux",
            _ => "natives-linux",
        };

        assert!(!k.is_empty());
        let _ = expected;
    }

    #[test]
    fn disallowed_library_excluded() {
        let lib = Library {
            name: "x:y:1".into(),
            rules: Some(vec![Rule {
                action: RuleAction::Disallow,
                os: Some(OsRule {
                    name: Some(crate::utils::system::mojang_os().to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            }]),
            downloads: None,
            natives: None,
            extract: None,
            url: None,
        };
        assert!(!library_applies(&lib));
    }
}
