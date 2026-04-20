use serde::{Deserialize, Serialize};

/// In-memory entity tag record.
/// Raw format in tags.json is { "uuid": "name", ... }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTag {
    pub uuid: String,
    pub name: String,
}
