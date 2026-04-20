use serde::{Deserialize, Serialize};

/// In-memory resource type record from scunpacked-data resource-types.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceType {
    pub uuid: String,
    pub key: String,
    pub name: String,
    pub description: String,
    pub refined_version_uuid: Option<String>,
    pub validate_default_cargo_box: bool,
    pub has_default_cargo_containers: bool,
    pub box_sizes_scu: Vec<f64>,
    pub data: serde_json::Value,
}

impl ResourceType {
    pub fn from_payload(payload: &serde_json::Value) -> Option<Self> {
        let uuid = payload.get("uuid")?.as_str()?.trim().to_string();
        let key = payload.get("key")?.as_str()?.trim().to_string();

        if uuid.is_empty() || key.is_empty() {
            return None;
        }

        let box_sizes: Vec<f64> = payload.get("box_sizes_scu")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();

        Some(ResourceType {
            uuid,
            key,
            name: payload.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            description: payload.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            refined_version_uuid: payload.get("refined_version_uuid")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            validate_default_cargo_box: payload.get("validate_default_cargo_box")
                .and_then(|v| v.as_bool()).unwrap_or(false),
            has_default_cargo_containers: payload.get("has_default_cargo_containers")
                .and_then(|v| v.as_bool()).unwrap_or(false),
            box_sizes_scu: box_sizes,
            data: payload.clone(),
        })
    }
}
