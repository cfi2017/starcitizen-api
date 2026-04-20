use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw vehicle JSON from scunpacked-data ships/*.json
#[derive(Debug, Deserialize, Serialize)]
pub struct VehicleFilePayload {
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
    #[serde(rename = "ClassName")]
    pub class_name: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Career")]
    pub career: Option<String>,
    #[serde(rename = "Role")]
    pub role: Option<String>,
    #[serde(rename = "IsVehicle")]
    pub is_vehicle: Option<bool>,
    #[serde(rename = "IsGravlev")]
    pub is_gravlev: Option<bool>,
    #[serde(rename = "IsSpaceship")]
    pub is_spaceship: Option<bool>,
    #[serde(rename = "Size")]
    pub size: Option<serde_json::Value>,
    #[serde(rename = "Manufacturer")]
    pub manufacturer: Option<ManufacturerRef>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ManufacturerRef {
    #[serde(rename = "UUID")]
    pub uuid: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

/// In-memory vehicle record
#[derive(Debug, Clone, Serialize)]
pub struct Vehicle {
    pub uuid: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub class_name: Option<String>,
    pub career: Option<String>,
    pub role: Option<String>,
    pub is_vehicle: bool,
    pub is_gravlev: bool,
    pub is_spaceship: bool,
    pub size: Option<serde_json::Value>,
    pub manufacturer_uuid: Option<String>,
    /// Full payload data
    pub data: serde_json::Value,
    pub translations: HashMap<String, String>,
}

impl Vehicle {
    pub fn from_payload(payload: VehicleFilePayload) -> Option<Self> {
        let uuid = payload.uuid.as_deref()?.trim().to_string();
        if uuid.is_empty() {
            return None;
        }

        let manufacturer_uuid = payload.manufacturer.as_ref()
            .and_then(|m| m.uuid.clone());

        let name = payload.name.clone();
        let display_name = generate_display_name(
            name.as_deref(),
            payload.manufacturer.as_ref().and_then(|m| m.name.as_deref()),
        );

        // Reconstruct the full data as a Value
        let data = serde_json::to_value(&payload).unwrap_or(serde_json::Value::Null);

        Some(Vehicle {
            uuid,
            name: payload.name,
            display_name,
            class_name: payload.class_name,
            career: payload.career,
            role: payload.role,
            is_vehicle: payload.is_vehicle.unwrap_or(false),
            is_gravlev: payload.is_gravlev.unwrap_or(false),
            is_spaceship: payload.is_spaceship.unwrap_or(false),
            size: payload.size,
            manufacturer_uuid,
            data,
            translations: HashMap::new(),
        })
    }
}

fn generate_display_name(name: Option<&str>, manufacturer_name: Option<&str>) -> Option<String> {
    let name = name?.trim();
    if name.is_empty() {
        return None;
    }

    let normalized = name.replace('_', " ");
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    let Some(mfr) = manufacturer_name else {
        return Some(normalized);
    };

    let special_cases: &[(&str, &[&str])] = &[
        ("Roberts Space Industries", &["RSI"]),
        ("Consolidated Outland", &["C.O."]),
        ("Musashi Industrial & Starflight Concern", &["MISC"]),
    ];

    let mut candidates: Vec<&str> = Vec::new();
    for (full_name, shorts) in special_cases {
        if *full_name == mfr {
            candidates.extend_from_slice(shorts);
        }
    }
    candidates.push(mfr);
    if let Some(first_word) = mfr.split_whitespace().next() {
        if !candidates.contains(&first_word) {
            candidates.push(first_word);
        }
    }

    for prefix in &candidates {
        if let Some(stripped) = normalized.strip_prefix(prefix) {
            let stripped = stripped.trim();
            if !stripped.is_empty() {
                return Some(stripped.to_string());
            }
        }
        // Case-insensitive check
        let lower = normalized.to_lowercase();
        let prefix_lower = prefix.to_lowercase();
        if lower.starts_with(&prefix_lower) {
            let stripped = &normalized[prefix.len()..].trim();
            if !stripped.is_empty() {
                return Some(stripped.to_string());
            }
        }
    }

    Some(normalized)
}
