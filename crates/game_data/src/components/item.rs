use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub name: String,
    pub namespace: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub components: HashMap<String, Value>,
}

impl ItemDef {
    pub fn qualified_id(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }
}
