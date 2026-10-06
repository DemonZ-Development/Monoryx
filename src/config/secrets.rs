use super::LauncherConfig;
use crate::error::{MonoryxError, Result};

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Secrets {
    access_token: String,
    refresh_token: String,
    curseforge_key: String,
}

impl Secrets {
    pub fn from_config(config: &LauncherConfig) -> Self {
        Self {
            access_token: config
                .microsoft_profile
                .as_ref()
                .map(|p| p.access_token.clone())
                .unwrap_or_default(),
            refresh_token: config
                .microsoft_profile
                .as_ref()
                .map(|p| p.refresh_token.clone())
                .unwrap_or_default(),
            curseforge_key: config.curseforge.api_key.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.access_token.is_empty()
            && self.refresh_token.is_empty()
            && self.curseforge_key.is_empty()
    }

    pub fn apply(&self, config: &mut LauncherConfig) {
        if let Some(profile) = &mut config.microsoft_profile {
            profile.access_token.clone_from(&self.access_token);
            profile.refresh_token.clone_from(&self.refresh_token);
        }
        config.curseforge.api_key.clone_from(&self.curseforge_key);
    }

    pub fn read(id: &str) -> Result<Self> {
        Ok(Self {
            access_token: backend::read(id, "access")?,
            refresh_token: backend::read(id, "refresh")?,
            curseforge_key: backend::read(id, "curseforge")?,
        })
    }

    pub fn write(&self, id: &str) -> Result<()> {
        for (name, value) in [
            ("access", &self.access_token),
            ("refresh", &self.refresh_token),
            ("curseforge", &self.curseforge_key),
        ] {
            backend::write(id, name, value)?;
        }
        Ok(())
    }

    pub fn delete(id: &str) {
        for name in ["access", "refresh", "curseforge"] {
            if let Err(error) = backend::delete(id, name) {
                tracing::warn!("Could not remove saved credential: {error}");
            }
        }
    }
}

#[cfg(not(test))]
mod backend {
    use super::*;

    fn entry(id: &str, name: &str) -> Result<keyring::Entry> {
        keyring::Entry::new("MONORYX", &format!("{id}-{name}")).map_err(vault_error)
    }

    fn vault_error(error: keyring::Error) -> MonoryxError {
        MonoryxError::Auth(format!("The operating system credential store is unavailable: {error}. Existing settings have been preserved."))
    }

    pub fn read(id: &str, name: &str) -> Result<String> {
        let bytes = entry(id, name)?.get_secret().map_err(vault_error)?;
        String::from_utf8(bytes).map_err(|_| {
            MonoryxError::Auth(
                "Saved credential has invalid encoding. Existing settings have been preserved."
                    .into(),
            )
        })
    }

    pub fn write(id: &str, name: &str, value: &str) -> Result<()> {
        entry(id, name)?
            .set_secret(value.as_bytes())
            .map_err(vault_error)
    }

    pub fn delete(id: &str, name: &str) -> Result<()> {
        match entry(id, name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(vault_error(error)),
        }
    }
}

#[cfg(test)]
pub(super) mod backend {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    thread_local! {
        pub static FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    fn values() -> &'static Mutex<HashMap<String, String>> {
        static VALUES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        VALUES.get_or_init(Mutex::default)
    }

    fn check_available() -> Result<()> {
        if FAIL.get() {
            Err(MonoryxError::Auth("Credential store unavailable".into()))
        } else {
            Ok(())
        }
    }

    pub fn read(id: &str, name: &str) -> Result<String> {
        check_available()?;
        values()
            .lock()
            .unwrap()
            .get(&format!("{id}-{name}"))
            .cloned()
            .ok_or_else(|| MonoryxError::Auth("Saved credential is missing".into()))
    }

    pub fn write(id: &str, name: &str, value: &str) -> Result<()> {
        check_available()?;
        values()
            .lock()
            .unwrap()
            .insert(format!("{id}-{name}"), value.into());
        Ok(())
    }

    pub fn delete(id: &str, name: &str) -> Result<()> {
        values().lock().unwrap().remove(&format!("{id}-{name}"));
        Ok(())
    }
}
