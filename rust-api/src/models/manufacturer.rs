use serde::{Deserialize, Serialize};

/// Raw manufacturer entry from scunpacked-data manufacturers.json
#[derive(Debug, Deserialize)]
pub struct ManufacturerRaw {
    pub reference: Option<String>,
    pub name: Option<String>,
    pub code: Option<String>,
}

/// In-memory manufacturer record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manufacturer {
    pub uuid: String,
    pub name: String,
    pub code: String,
}
