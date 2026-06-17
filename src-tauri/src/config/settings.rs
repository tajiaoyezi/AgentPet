//! `settings.json` placeholder bootstrap.

use std::fs;

use anyhow::{Context, Result};

use super::paths::AppPaths;

/// Schema marker written into the placeholder settings file.
pub const SETTINGS_SCHEMA: &str = "agentpet.settings/v1";

/// Write a minimal `settings.json` if it does not exist. Existing files are
/// preserved (never overwritten).
pub fn ensure_placeholder(paths: &AppPaths) -> Result<()> {
    let file = paths.settings_file();
    if file.exists() {
        return Ok(());
    }
    let value = serde_json::json!({ "schema": SETTINGS_SCHEMA });
    let text = serde_json::to_string_pretty(&value)?;
    fs::write(&file, text)
        .with_context(|| format!("写入 settings.json 失败: {}", file.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_created_then_preserved() {
        let tmp = std::env::temp_dir().join(format!("agentpet-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let paths = AppPaths::with_root(tmp.clone());

        ensure_placeholder(&paths).expect("create placeholder");
        let written = fs::read_to_string(paths.settings_file()).unwrap();
        assert!(written.contains(SETTINGS_SCHEMA));

        // Simulate a user edit; a second call must not overwrite it.
        fs::write(
            paths.settings_file(),
            "{\"schema\":\"agentpet.settings/v1\",\"custom\":1}",
        )
        .unwrap();
        ensure_placeholder(&paths).expect("idempotent");
        let after = fs::read_to_string(paths.settings_file()).unwrap();
        assert!(after.contains("custom"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
