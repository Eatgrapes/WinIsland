use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "github-link")]
    pub github_link: String,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("'name' is empty".into());
        }
        if self.author.trim().is_empty() {
            return Err("'author' is empty".into());
        }
        if self.version.trim().is_empty() {
            return Err("'version' is empty".into());
        }
        if self.description.trim().is_empty() {
            return Err("'description' is empty".into());
        }
        if self.github_link.trim().is_empty() {
            return Err("'github-link' is empty".into());
        }
        Ok(())
    }

    pub fn safe_dir_name(&self) -> String {
        self.name
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
}

fn read_zip_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let idx = zip.index_for_name(name)?;
    let mut entry = zip.by_index(idx).ok()?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub fn validate_zip(zip_path: &Path) -> Result<bool, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;

    let has_yml = zip.index_for_name("plugin.yml").is_some();
    let has_dll = zip.file_names().any(|n| n.ends_with(".dll"));

    if !has_yml {
        return Err("Missing plugin.yml in zip".into());
    }
    if !has_dll {
        return Err("Missing .dll in zip".into());
    }
    Ok(true)
}

pub fn read_manifest_from_zip(zip_path: &Path) -> Result<PluginManifest, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;

    let yml_bytes = read_zip_entry(&mut zip, "plugin.yml")
        .ok_or_else(|| "plugin.yml not found in zip".to_string())?;

    let manifest: PluginManifest =
        serde_yaml::from_slice(&yml_bytes).map_err(|e| format!("Invalid plugin.yml: {}", e))?;

    manifest.validate()?;
    Ok(manifest)
}

pub fn extract_plugin(
    zip_path: &Path,
    plugin_dir: &Path,
) -> Result<(PluginManifest, PathBuf, Vec<String>), String> {
    let manifest = read_manifest_from_zip(zip_path)?;
    let dir_name = manifest.safe_dir_name();
    let dest = plugin_dir.join(&dir_name);

    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .map_err(|e| format!("Cannot remove existing plugin dir: {}", e))?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| format!("Cannot create plugin dir: {}", e))?;

    let file = std::fs::File::open(zip_path).map_err(|e| format!("Cannot open zip: {}", e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip: {}", e))?;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Zip read error: {}", e))?;
        let name = entry.name().to_string();
        let out_path = dest.join(&name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).ok();
        } else {
            let mut out = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create {}: {}", name, e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Cannot extract {}: {}", name, e))?;
        }
    }

    let dll_paths = std::fs::read_dir(&dest)
        .map_err(|e| format!("Cannot read plugin dir: {}", e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "dll"))
        .collect::<Vec<_>>();

    let dll_strs = dll_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    Ok((manifest, dest, dll_strs))
}
