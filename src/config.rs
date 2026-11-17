use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Keys {
    pub key: String,
    pub secret: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Credentials {
    pub consumer: Keys,
    pub oauth_token: Option<Keys>,
}

pub fn default_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("plurk-cli")
        .join("key.toml")
}

fn io_err(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.display().to_string(),
        source,
    }
}

impl Credentials {
    pub fn new(consumer: Keys, oauth_token: Option<Keys>) -> Self {
        Self {
            consumer,
            oauth_token,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|e| io_err(path, e))?;
        toml::from_str(&text).map_err(|source| Error::ConfigParse {
            path: path.display().to_string(),
            source,
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
        }
        let text = toml::to_string(self)?;
        fs::write(path, text).map_err(|e| io_err(path, e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .map_err(|e| io_err(path, e))?;
        }
        Ok(())
    }

    pub fn token(&self) -> Option<&Keys> {
        self.oauth_token
            .as_ref()
            .filter(|k| !k.key.is_empty() && !k.secret.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Credentials {
        Credentials::new(
            Keys {
                key: "ck".into(),
                secret: "cs".into(),
            },
            None,
        )
    }

    #[test]
    fn loads_existing_format() {
        let dir = tempdir("load");
        let p = dir.join("key.toml");
        fs::write(
            &p,
            "[consumer]\nkey = \"a\"\nsecret = \"b\"\n\n[oauth_token]\nkey = \"\"\nsecret = \"\"\n",
        )
        .unwrap();
        let c = Credentials::load(&p).unwrap();
        assert_eq!(c.consumer.key, "a");
        assert!(c.token().is_none());
    }

    #[test]
    fn save_creates_parent_and_roundtrips_with_0600() {
        let dir = tempdir("save");
        let p = dir.join("nested").join("key.toml");
        let mut c = sample();
        c.oauth_token = Some(Keys {
            key: "tk".into(),
            secret: "ts".into(),
        });
        c.save(&p).unwrap();
        assert_eq!(Credentials::load(&p).unwrap(), c);
        assert!(c.token().is_some());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn default_path_has_no_side_effects() {
        let p = default_path();
        assert!(p.ends_with("plurk-cli/key.toml"));
    }

    fn tempdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("plurk-cli-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }
}
