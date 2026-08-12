use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::components::item::ItemDef;

pub const REGISTRY_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameRegistry {
    /// Bump when defs change so clients know to resync.
    pub version: u32,
    pub items: HashMap<String, ItemDef>,
}

impl GameRegistry {
    pub fn item(&self, id: &str) -> Option<&ItemDef> {
        self.items.get(id)
    }

    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let mut registry = GameRegistry {
            version: REGISTRY_VERSION,
            ..Default::default()
        };

        for (name, bytes) in read_json_files(&dir.join("items"))? {
            let def: ItemDef = serde_json::from_slice(&bytes)
                .with_context(|| format!("parsing item def '{name}'"))?;
            registry.items.insert(def.qualified_id(), def);
        }

        Ok(registry)
    }
}

fn read_json_files(dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            files.push((name, bytes));
        }
    }
    Ok(files)
}
