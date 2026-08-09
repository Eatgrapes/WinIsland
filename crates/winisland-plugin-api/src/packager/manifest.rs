use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a `plugin.yml` manifest for a WinIsland plugin.
///
/// This struct is serialised to YAML when packaging a plugin,
/// and deserialised by the WinIsland host when loading a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "github-link")]
    pub github_link: String,
    #[serde(rename = "abi-version")]
    pub abi_version: u32,
    pub entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dll_hashes: Option<Vec<String>>,
}

impl PluginManifest {
    /// Build the signing payload: canonical JSON of all fields except `signature`.
    pub fn signing_payload(&self) -> String {
        let payload = serde_json::json!({
            "id": self.id,
            "name": self.name,
            "author": self.author,
            "version": self.version,
            "description": self.description,
            "github-link": self.github_link,
            "abi-version": self.abi_version,
            "entry": self.entry,
            "dll_hashes": self.dll_hashes,
        });
        serde_json::to_string(&payload).unwrap_or_default()
    }

    /// Write the manifest to a `plugin.yml` file.
    pub fn write_to_yaml(&self, path: &Path) -> Result<(), String> {
        let yaml = serde_yaml::to_string(self)
            .map_err(|e| format!("Failed to serialise manifest: {}", e))?;
        std::fs::write(path, &yaml).map_err(|e| format!("Failed to write manifest: {}", e))
    }

    /// Compute a safe directory name from the plugin name.
    pub fn safe_dir_name(&self) -> String {
        self.id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Validate required fields are non-empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 63
            || !self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err("'id' must be at most 63 bytes and match [a-zA-Z0-9_-]+".into());
        }
        validate_text("name", &self.name, 127)?;
        validate_text("author", &self.author, 127)?;
        validate_text("version", &self.version, 31)?;
        validate_text("description", &self.description, 255)?;
        validate_text("github-link", &self.github_link, 2048)?;
        if self.abi_version != crate::ABI_VERSION_1 {
            return Err(format!("'abi-version' must be {}", crate::ABI_VERSION_1));
        }
        let entry = Path::new(&self.entry);
        if self.entry.is_empty()
            || self.entry.len() > 255
            || entry.components().count() != 1
            || !entry
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        {
            return Err("'entry' must be a root-level .dll filename".into());
        }
        Ok(())
    }
}

fn validate_text(field: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("'{field}' is empty"));
    }
    if value.len() > max_bytes {
        return Err(format!("'{field}' exceeds {max_bytes} UTF-8 bytes"));
    }
    Ok(())
}
