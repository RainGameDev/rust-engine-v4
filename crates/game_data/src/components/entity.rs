use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A placeable prop/entity definition. References a 3D model (glTF) and
/// optionally ships default component data applied to every placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    pub name: String,
    pub namespace: String,
    /// Path to the model relative to a registered asset dir, e.g. `meshes/tree.glb`.
    pub model: String,
    #[serde(default)]
    pub components: HashMap<String, Value>,
}

impl EntityDef {
    pub fn qualified_id(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}
