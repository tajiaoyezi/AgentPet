//! Application data paths.
//!
//! Resolves the AgentPet data root (Windows: `%APPDATA%\AgentPet`) via the
//! cross-platform `directories` crate and idempotently creates the directory
//! tree. Intentionally Tauri-free so a future standalone sidecar can reuse it
//! (design D4).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::BaseDirs;

/// Resolved AgentPet data paths, rooted at the per-user config directory.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
}

impl AppPaths {
    /// Resolve the data root from the OS config dir (Windows: `%APPDATA%`,
    /// macOS: `~/Library/Application Support`).
    pub fn resolve() -> Result<Self> {
        let base = BaseDirs::new().context("无法解析用户目录 (directories::BaseDirs)")?;
        Ok(Self::with_root(base.config_dir().join("AgentPet")))
    }

    /// Build paths around an explicit root (used by tests).
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn hooks_dir(&self) -> PathBuf {
        self.root.join("hooks")
    }
    pub fn pets_dir(&self) -> PathBuf {
        self.root.join("pets")
    }
    pub fn sounds_dir(&self) -> PathBuf {
        self.root.join("sounds")
    }
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    /// Create the data directory tree if missing. Idempotent: existing
    /// directories and their contents are left untouched.
    pub fn ensure_tree(&self) -> Result<()> {
        for dir in [
            self.root.clone(),
            self.logs_dir(),
            self.hooks_dir(),
            self.pets_dir(),
            self.sounds_dir(),
            self.bin_dir(),
        ] {
            fs::create_dir_all(&dir)
                .with_context(|| format!("创建目录失败: {}", dir.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_tree_is_idempotent_and_nondestructive() {
        let tmp = std::env::temp_dir().join(format!("agentpet-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let paths = AppPaths::with_root(tmp.clone());

        // First run creates the full tree.
        paths.ensure_tree().expect("first ensure_tree");
        for d in [
            paths.logs_dir(),
            paths.hooks_dir(),
            paths.pets_dir(),
            paths.sounds_dir(),
            paths.bin_dir(),
        ] {
            assert!(d.is_dir(), "expected dir to exist: {}", d.display());
        }

        // Drop a marker, re-run: must not error or clobber existing contents.
        let marker = paths.pets_dir().join("marker.txt");
        fs::write(&marker, "keep").unwrap();
        paths.ensure_tree().expect("second ensure_tree");
        assert_eq!(fs::read_to_string(&marker).unwrap(), "keep");

        let _ = fs::remove_dir_all(&tmp);
    }
}
