use serde::{Deserialize, Serialize};

/// In-memory blueprint record.
/// Raw data comes from scunpacked-data blueprints.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub uuid: String,
    pub key: String,
    pub category_uuid: String,
    pub output_item_uuid: Option<String>,
    pub output_name: Option<String>,
    pub output_class: Option<String>,
    pub craft_time_seconds: Option<i64>,
    pub is_available_by_default: bool,
    pub ingredient_resource_type_uuids: Vec<String>,
    /// Full blueprint payload data
    pub data: serde_json::Value,
}

impl Blueprint {
    pub fn from_payload(payload: &serde_json::Value) -> Option<Self> {
        let uuid = payload.get("uuid")?.as_str()?.trim().to_string();
        let key = payload.get("key")?.as_str()?.trim().to_string();
        let category_uuid = payload.get("category_uuid")?.as_str()?.trim().to_string();
        let output_uuid = payload.pointer("/output/uuid")?.as_str()?.trim().to_string();

        if uuid.is_empty() || key.is_empty() || category_uuid.is_empty() || output_uuid.is_empty() {
            return None;
        }

        let output_name = payload.pointer("/output/name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let output_class = payload.pointer("/output/class")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let craft_time_seconds = payload.pointer("/tiers/0/craft_time_seconds")
            .and_then(|v| v.as_i64());

        let is_available_by_default = payload.pointer("/availability/default")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ingredient_resource_type_uuids = collect_ingredient_uuids(payload);

        Some(Blueprint {
            uuid,
            key,
            category_uuid,
            output_item_uuid: Some(output_uuid),
            output_name,
            output_class,
            craft_time_seconds,
            is_available_by_default,
            ingredient_resource_type_uuids,
            data: payload.clone(),
        })
    }
}

fn collect_ingredient_uuids(payload: &serde_json::Value) -> Vec<String> {
    let mut uuids = std::collections::HashSet::new();

    if let Some(tiers) = payload.get("tiers").and_then(|v| v.as_array()) {
        for tier in tiers {
            if let Some(requirements) = tier.get("requirements") {
                collect_resource_uuids(requirements, &mut uuids);
            }
        }
    }

    uuids.into_iter().collect()
}

fn collect_resource_uuids(node: &serde_json::Value, uuids: &mut std::collections::HashSet<String>) {
    if let Some(kind) = node.get("kind").and_then(|v| v.as_str()) {
        if kind == "resource" {
            if let Some(uuid) = node.get("uuid").and_then(|v| v.as_str()) {
                let uuid = uuid.trim().to_string();
                if !uuid.is_empty() {
                    uuids.insert(uuid);
                }
            }
            return;
        }
    }

    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            collect_resource_uuids(child, uuids);
        }
    }
}
