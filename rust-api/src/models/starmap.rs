use serde::{Deserialize, Serialize};

/// In-memory starmap location from scunpacked-data starmap.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarmapLocation {
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    pub type_name: String,
    pub size: Option<f64>,
    pub is_scannable: bool,
    pub block_travel: bool,
    pub parent_uuid: Option<String>,
    pub system: Option<String>,
    pub amenities: Vec<Amenity>,
    /// Full entry data
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amenity {
    pub uuid: String,
    pub name: String,
    pub display_name: Option<String>,
}

impl StarmapLocation {
    pub fn from_payload(entry: &serde_json::Value) -> Option<Self> {
        let uuid = entry.get("uuid")?.as_str()?.trim().to_string();
        if uuid.is_empty() {
            return None;
        }

        let name = entry.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| uuid.clone());

        let type_name = entry.pointer("/type/name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());

        let amenities = entry.get("amenities")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        let uuid = a.get("uuid")?.as_str()?.trim().to_string();
                        if uuid.is_empty() { return None; }
                        let name = a.get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| uuid.clone());
                        let display_name = a.get("displayName")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty());
                        Some(Amenity { uuid, name, display_name })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let parent_uuid = entry.get("parentUuid")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let description = entry.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Some(StarmapLocation {
            uuid,
            name,
            description,
            type_name,
            size: entry.get("size").and_then(|v| v.as_f64()),
            is_scannable: entry.get("isScannable").and_then(|v| v.as_bool()).unwrap_or(false),
            block_travel: entry.get("blockTravel").and_then(|v| v.as_bool()).unwrap_or(false),
            parent_uuid,
            system: None, // resolved after loading all locations
            amenities,
            data: entry.clone(),
        })
    }
}
