use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// In-memory label with translations.
/// English labels come from labels.json, German from global.ini, Chinese from global.ini.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLabel {
    pub key: String,
    pub translations: HashMap<String, String>,
}
