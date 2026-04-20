use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw item JSON from scunpacked-data items/*.json
#[derive(Debug, Deserialize)]
pub struct ItemFilePayload {
    #[serde(rename = "Item")]
    pub item: Option<serde_json::Value>,
    #[serde(rename = "Raw")]
    pub raw: Option<serde_json::Value>,
}

/// In-memory item record
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub uuid: String,
    pub name: String,
    pub class_name: Option<String>,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub sub_type: Option<String>,
    pub classification: Option<String>,
    pub size: Option<i64>,
    pub grade: Option<i64>,
    pub class: Option<String>,
    pub manufacturer_uuid: Option<String>,
    /// Full Item section from the JSON (arbitrary nested data)
    pub data: serde_json::Value,
    /// Translations keyed by locale
    pub translations: HashMap<String, String>,
    /// Entity tag UUIDs
    pub entity_tags: Vec<String>,
}

impl Item {
    pub fn from_payload(item_val: &serde_json::Value, raw_val: Option<&serde_json::Value>) -> Option<Self> {
        let uuid = item_val.get("reference")
            .or_else(|| item_val.get("uuid"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())?;

        let name = ["name", "itemName", "className"]
            .iter()
            .find_map(|key| {
                item_val.get(key)
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "Unknown Item".to_string());

        let manufacturer_uuid = item_val
            .pointer("/stdItem/Manufacturer/UUID")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let item_class = item_val
            .pointer("/stdItem/DescriptionData/Class")
            .and_then(|v| v.as_str())
            .filter(|c| ["Industrial", "Civilian", "Military", "Stealth", "Competition"].contains(c))
            .map(|s| s.to_string());

        let entity_tags = item_val
            .get("entity_tag_map")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tag| tag.get("tag").and_then(|t| t.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Extract English description from Raw payload
        let mut translations = HashMap::new();
        if let Some(desc) = extract_english_description(item_val, raw_val) {
            translations.insert("en".to_string(), desc);
        }

        Some(Item {
            uuid,
            name,
            class_name: item_val.get("className").and_then(|v| v.as_str()).map(|s| s.to_string()),
            item_type: item_val.get("type").and_then(|v| v.as_str()).map(|s| s.to_string()),
            sub_type: item_val.get("subType").and_then(|v| v.as_str()).map(|s| s.to_string()),
            classification: item_val.get("classification").and_then(|v| v.as_str()).map(|s| s.to_string()),
            size: item_val.get("size").and_then(|v| v.as_i64()),
            grade: item_val.get("grade").and_then(|v| v.as_i64()),
            class: item_class,
            manufacturer_uuid,
            data: item_val.clone(),
            translations,
            entity_tags,
        })
    }

    /// Resolve the description label key from Raw data for loading translations
    pub fn description_label(raw_val: Option<&serde_json::Value>) -> Option<String> {
        let raw = raw_val?;
        let attach_def = raw.pointer("/Entity/Components/SAttachableComponentParams/AttachDef")?;

        let label = attach_def.get("Localization__Description")
            .or_else(|| attach_def.pointer("/Localization/__Description"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s.to_lowercase() != "@loc_empty")?;

        Some(label.trim_start_matches('@').to_string())
    }
}

fn extract_english_description(
    item_val: &serde_json::Value,
    raw_val: Option<&serde_json::Value>,
) -> Option<String> {
    // Try multiple sources for the English description
    let candidates = [
        item_val.pointer("/stdItem/DescriptionText").and_then(|v| v.as_str()),
        item_val.pointer("/stdItem/Description").and_then(|v| v.as_str()),
    ];

    for candidate in candidates {
        if let Some(desc) = candidate {
            let trimmed = desc.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // Try from Raw localization
    if let Some(raw) = raw_val {
        let attach_def = raw.pointer("/Entity/Components/SAttachableComponentParams/AttachDef");
        if let Some(def) = attach_def {
            let localization_candidates = [
                def.pointer("/Localization/English/Description"),
                def.pointer("/Localization/Description"),
            ];

            for candidate in localization_candidates {
                if let Some(desc) = candidate.and_then(|v| v.as_str()) {
                    let trimmed = desc.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }
    }

    None
}
