use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bump when the level format changes so loaders can reject old files.
pub const LEVEL_VERSION: u32 = 1;

/// Serializable world-space transform. Rotation is stored as Y-up Euler
/// degrees so the JSON stays human editable; the editor converts it to the
/// engine's `UnitQuaternion` when spawning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelTransform {
    #[serde(default)]
    pub position: [f32; 3],
    /// Euler rotation in degrees (applied in XYZ order).
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default = "one_scale")]
    pub scale: [f32; 3],
}

impl Default for LevelTransform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl LevelTransform {
    pub fn from_position(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn position_vec(&self) -> nalgebra::Vector3<f32> {
        self.position.into()
    }
}

fn one_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

/// A place where players can spawn into the level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnPoint {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub transform: LevelTransform,
}

/// A single prop/entity placed in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedEntity {
    #[serde(default)]
    pub name: String,
    /// Qualified id of the `EntityDef` this placement references, e.g. `trees:oak`.
    pub def_id: String,
    #[serde(default)]
    pub transform: LevelTransform,
    /// Per-instance component overrides, keyed by component type name.
    #[serde(default)]
    pub components: HashMap<String, Value>,
}

/// A complete editable world/level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Level {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub spawn_points: Vec<SpawnPoint>,
    #[serde(default)]
    pub entities: Vec<PlacedEntity>,
}

impl Level {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: LEVEL_VERSION,
            name: name.into(),
            description: String::new(),
            spawn_points: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let level: Level = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing level '{}'", path.display()))?;
        if level.version != LEVEL_VERSION {
            anyhow::bail!(
                "level '{}' has unsupported version {} (expected {})",
                path.display(),
                level.version,
                LEVEL_VERSION
            );
        }
        Ok(level)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}
