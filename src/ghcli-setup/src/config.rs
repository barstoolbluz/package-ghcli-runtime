use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Key=value config file reader/writer with atomic operations.
pub struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Get the path to the config file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read all key-value pairs from the config file.
    fn read_all(&self) -> Result<Vec<(String, String)>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("reading config file {}", self.path.display()))?;
        let mut pairs = Vec::new();
        for line in content.lines() {
            if let Some(idx) = line.find('=') {
                let key = line[..idx].to_string();
                let value = line[idx + 1..].to_string();
                pairs.push((key, value));
            }
        }
        Ok(pairs)
    }

    /// Get a config value by key. Returns None if not found.
    pub fn get(&self, key: &str) -> Result<Option<String>> {
        let pairs = self.read_all()?;
        // Return the last matching key (matches bash `tail -n1` behavior)
        Ok(pairs
            .into_iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v))
    }

    /// Set a config key to a value (replaces existing, appends if new). Atomic write.
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        self.ensure_dir()?;
        let pairs = self.read_all()?;
        let mut tmp = NamedTempFile::new_in(self.path.parent().context("config path has no parent")?)
            .context("creating temp file for config")?;

        use std::io::Write;
        // Write all lines except matching key
        for (k, v) in &pairs {
            if k != key {
                writeln!(tmp, "{}={}", k, v)?;
            }
        }
        // Append new value
        writeln!(tmp, "{}={}", key, value)?;

        set_permissions(tmp.path(), 0o600)?;
        tmp.persist(&self.path)
            .with_context(|| format!("persisting config to {}", self.path.display()))?;
        Ok(())
    }

    /// Remove a key from the config file. Atomic write.
    pub fn unset(&self, key: &str) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        self.ensure_dir()?;
        let pairs = self.read_all()?;
        let mut tmp = NamedTempFile::new_in(self.path.parent().context("config path has no parent")?)
            .context("creating temp file for config unset")?;

        use std::io::Write;
        for (k, v) in &pairs {
            if k != key {
                writeln!(tmp, "{}={}", k, v)?;
            }
        }

        set_permissions(tmp.path(), 0o600)?;
        tmp.persist(&self.path)
            .with_context(|| format!("persisting config to {}", self.path.display()))?;
        Ok(())
    }

    fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
            set_permissions(parent, 0o700).ok();
        }
        Ok(())
    }
}

/// Set Unix permissions on a path.
pub fn set_permissions(path: &Path, mode: u32) -> Result<()> {
    let perms = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, perms)
        .with_context(|| format!("setting permissions on {}", path.display()))?;
    Ok(())
}

/// Atomically write content to a file with given permissions.
pub fn atomic_write(target: &Path, content: &[u8], mode: u32) -> Result<()> {
    let parent = target
        .parent()
        .context("target path has no parent directory")?;
    fs::create_dir_all(parent)?;
    set_permissions(parent, 0o700).ok();

    let mut tmp =
        NamedTempFile::new_in(parent).context("creating temp file for atomic write")?;
    std::io::Write::write_all(&mut tmp, content)?;
    set_permissions(tmp.path(), mode)?;
    tmp.persist(target)
        .with_context(|| format!("persisting to {}", target.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_config");
        let cfg = ConfigFile::new(path.clone());

        cfg.set("FOO", "bar").unwrap();
        assert_eq!(cfg.get("FOO").unwrap(), Some("bar".to_string()));

        cfg.set("FOO", "baz").unwrap();
        assert_eq!(cfg.get("FOO").unwrap(), Some("baz".to_string()));

        cfg.unset("FOO").unwrap();
        assert_eq!(cfg.get("FOO").unwrap(), None);
    }

    #[test]
    fn test_config_multiple_keys() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_config");
        let cfg = ConfigFile::new(path);

        cfg.set("A", "1").unwrap();
        cfg.set("B", "2").unwrap();
        cfg.set("C", "3").unwrap();

        assert_eq!(cfg.get("A").unwrap(), Some("1".to_string()));
        assert_eq!(cfg.get("B").unwrap(), Some("2".to_string()));
        assert_eq!(cfg.get("C").unwrap(), Some("3".to_string()));

        cfg.unset("B").unwrap();
        assert_eq!(cfg.get("B").unwrap(), None);
        assert_eq!(cfg.get("A").unwrap(), Some("1".to_string()));
        assert_eq!(cfg.get("C").unwrap(), Some("3".to_string()));
    }

    #[test]
    fn test_get_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent");
        let cfg = ConfigFile::new(path);
        assert_eq!(cfg.get("FOO").unwrap(), None);
    }
}
