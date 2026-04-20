use crate::models::{
    blueprint::Blueprint,
    item::{Item, ItemFilePayload},
    label::GameLabel,
    manufacturer::{Manufacturer, ManufacturerRaw},
    resource_type::ResourceType,
    starmap::StarmapLocation,
    tag::EntityTag,
    vehicle::{Vehicle, VehicleFilePayload},
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// All game data loaded into memory.
pub struct GameData {
    pub items: Vec<Item>,
    pub vehicles: Vec<Vehicle>,
    pub blueprints: Vec<Blueprint>,
    pub manufacturers: Vec<Manufacturer>,
    pub resource_types: Vec<ResourceType>,
    pub starmap_locations: Vec<StarmapLocation>,
    pub entity_tags: Vec<EntityTag>,
    pub labels: HashMap<String, GameLabel>,

    // Indexes for fast lookup
    pub items_by_uuid: HashMap<String, usize>,
    pub items_by_name: HashMap<String, usize>,
    pub vehicles_by_uuid: HashMap<String, usize>,
    pub blueprints_by_uuid: HashMap<String, usize>,
    pub manufacturers_by_uuid: HashMap<String, usize>,
    pub manufacturers_by_code: HashMap<String, usize>,
    pub resource_types_by_uuid: HashMap<String, usize>,
    pub starmap_locations_by_uuid: HashMap<String, usize>,
    pub tags_by_uuid: HashMap<String, usize>,
}

impl GameData {
    pub fn load(data_dir: &Path) -> Self {
        let scunpacked = data_dir.join("scunpacked-data");
        let german_ini = data_dir.join("StarCitizenDeutsch/live/global.ini");
        let chinese_ini = data_dir.join("ScToolBoxLocales/chinese_(simplified)/global.ini");

        info!("Loading game data from {}", scunpacked.display());

        // Load labels first (needed for translations)
        let labels = load_labels(&scunpacked, &german_ini, &chinese_ini);
        info!("Loaded {} labels", labels.len());

        // Load manufacturers
        let manufacturers = load_manufacturers(&scunpacked);
        info!("Loaded {} manufacturers", manufacturers.len());

        // Load entity tags
        let entity_tags = load_entity_tags(&scunpacked);
        info!("Loaded {} entity tags", entity_tags.len());

        // Load resource types
        let resource_types = load_resource_types(&scunpacked);
        info!("Loaded {} resource types", resource_types.len());

        // Load blueprints
        let blueprints = load_blueprints(&scunpacked);
        info!("Loaded {} blueprints", blueprints.len());

        // Load items (parallel)
        let mut items = load_items(&scunpacked, &labels);
        info!("Loaded {} items", items.len());

        // Backfill translations from labels
        backfill_item_translations(&mut items, &scunpacked, &labels);

        // Load vehicles (parallel)
        let vehicles = load_vehicles(&scunpacked);
        info!("Loaded {} vehicles", vehicles.len());

        // Load starmap
        let starmap_locations = load_starmap(&scunpacked);
        info!("Loaded {} starmap locations", starmap_locations.len());

        // Build indexes
        let items_by_uuid = items.iter().enumerate()
            .map(|(i, item)| (item.uuid.clone(), i))
            .collect();
        let items_by_name: HashMap<String, usize> = items.iter().enumerate()
            .map(|(i, item)| (item.name.to_lowercase(), i))
            .collect();
        let vehicles_by_uuid = vehicles.iter().enumerate()
            .map(|(i, v)| (v.uuid.clone(), i))
            .collect();
        let blueprints_by_uuid = blueprints.iter().enumerate()
            .map(|(i, b)| (b.uuid.clone(), i))
            .collect();
        let manufacturers_by_uuid = manufacturers.iter().enumerate()
            .map(|(i, m)| (m.uuid.clone(), i))
            .collect();
        let manufacturers_by_code: HashMap<String, usize> = manufacturers.iter().enumerate()
            .filter(|(_, m)| !m.code.is_empty())
            .map(|(i, m)| (m.code.to_lowercase(), i))
            .collect();
        let resource_types_by_uuid = resource_types.iter().enumerate()
            .map(|(i, r)| (r.uuid.clone(), i))
            .collect();
        let starmap_locations_by_uuid = starmap_locations.iter().enumerate()
            .map(|(i, s)| (s.uuid.clone(), i))
            .collect();
        let tags_by_uuid = entity_tags.iter().enumerate()
            .map(|(i, t)| (t.uuid.clone(), i))
            .collect();

        GameData {
            items,
            vehicles,
            blueprints,
            manufacturers,
            resource_types,
            starmap_locations,
            entity_tags,
            labels,
            items_by_uuid,
            items_by_name,
            vehicles_by_uuid,
            blueprints_by_uuid,
            manufacturers_by_uuid,
            manufacturers_by_code,
            resource_types_by_uuid,
            starmap_locations_by_uuid,
            tags_by_uuid,
        }
    }

    pub fn memory_report(&self) -> String {
        format!(
            "Items: {}, Vehicles: {}, Blueprints: {}, Manufacturers: {}, \
             ResourceTypes: {}, StarmapLocations: {}, EntityTags: {}, Labels: {}",
            self.items.len(),
            self.vehicles.len(),
            self.blueprints.len(),
            self.manufacturers.len(),
            self.resource_types.len(),
            self.starmap_locations.len(),
            self.entity_tags.len(),
            self.labels.len(),
        )
    }
}

fn load_labels(
    scunpacked: &Path,
    german_ini: &Path,
    chinese_ini: &Path,
) -> HashMap<String, GameLabel> {
    let labels_path = scunpacked.join("labels.json");
    let english: HashMap<String, String> = match std::fs::read_to_string(&labels_path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(e) => {
            warn!("Could not read labels.json: {e}");
            HashMap::new()
        }
    };

    let german = read_ini_translations(german_ini);
    let chinese = read_ini_translations(chinese_ini);

    english
        .into_iter()
        .map(|(key, en_value)| {
            let mut translations = HashMap::new();
            translations.insert("en".to_string(), en_value);
            if let Some(de) = german.get(&key) {
                translations.insert("de".to_string(), de.clone());
            }
            if let Some(zh) = chinese.get(&key) {
                translations.insert("zh".to_string(), zh.clone());
            }

            let label = GameLabel {
                key: key.clone(),
                translations,
            };
            (key, label)
        })
        .collect()
}

fn read_ini_translations(path: &Path) -> HashMap<String, String> {
    if !path.exists() {
        return HashMap::new();
    }

    let mut parser = configparser::ini::Ini::new_cs();
    // INI files without sections - use default section
    match parser.load(path.to_string_lossy().as_ref()) {
        Ok(_) => {
            let mut result = HashMap::new();
            // Try the default section first
            for section in parser.sections() {
                if let Some(map) = parser.get_map_ref().get(&section) {
                    for (key, value) in map {
                        if let Some(val) = value {
                            result.insert(key.clone(), val.clone());
                        }
                    }
                }
            }
            result
        }
        Err(e) => {
            warn!("Could not parse INI {}: {e}", path.display());
            HashMap::new()
        }
    }
}

fn load_manufacturers(scunpacked: &Path) -> Vec<Manufacturer> {
    let path = scunpacked.join("manufacturers.json");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Could not read manufacturers.json: {e}");
            return vec![Manufacturer {
                uuid: "00000000-0000-0000-0000-000000000000".to_string(),
                name: "Unknown".to_string(),
                code: "UNKN".to_string(),
            }];
        }
    };

    let raw: Vec<ManufacturerRaw> = serde_json::from_str(&contents).unwrap_or_default();

    let mut result: Vec<Manufacturer> = raw
        .into_iter()
        .filter_map(|m| {
            let uuid = m.reference?.trim().to_string();
            let name = m.name?.trim().to_string();
            if uuid.is_empty() || name.is_empty() {
                return None;
            }
            Some(Manufacturer {
                uuid,
                name,
                code: m.code.unwrap_or_default().trim().to_string(),
            })
        })
        .collect();

    // Ensure "Unknown" manufacturer exists
    if !result.iter().any(|m| m.uuid == "00000000-0000-0000-0000-000000000000") {
        result.push(Manufacturer {
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            name: "Unknown".to_string(),
            code: "UNKN".to_string(),
        });
    }

    result
}

fn load_entity_tags(scunpacked: &Path) -> Vec<EntityTag> {
    let path = scunpacked.join("tags.json");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Could not read tags.json: {e}");
            return Vec::new();
        }
    };

    let raw: HashMap<String, String> = serde_json::from_str(&contents).unwrap_or_default();

    raw.into_iter()
        .filter(|(uuid, name)| !uuid.trim().is_empty() && !name.trim().is_empty())
        .map(|(uuid, name)| EntityTag {
            uuid: uuid.trim().to_string(),
            name: name.trim().to_string(),
        })
        .collect()
}

fn load_resource_types(scunpacked: &Path) -> Vec<ResourceType> {
    let path = scunpacked.join("resource-types.json");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Could not read resource-types.json: {e}");
            return Vec::new();
        }
    };

    let raw: Vec<serde_json::Value> = serde_json::from_str(&contents).unwrap_or_default();
    raw.iter().filter_map(ResourceType::from_payload).collect()
}

fn load_blueprints(scunpacked: &Path) -> Vec<Blueprint> {
    let path = scunpacked.join("blueprints.json");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Could not read blueprints.json: {e}");
            return Vec::new();
        }
    };

    let raw: Vec<serde_json::Value> = serde_json::from_str(&contents).unwrap_or_default();
    raw.iter().filter_map(Blueprint::from_payload).collect()
}

fn load_items(scunpacked: &Path, labels: &HashMap<String, GameLabel>) -> Vec<Item> {
    let items_dir = scunpacked.join("items");
    if !items_dir.is_dir() {
        warn!("Items directory not found: {}", items_dir.display());
        return Vec::new();
    }

    let pattern = items_dir.join("*.json").to_string_lossy().to_string();
    let paths: Vec<PathBuf> = glob::glob(&pattern)
        .map(|paths| paths.filter_map(Result::ok).collect())
        .unwrap_or_default();

    paths
        .par_iter()
        .filter_map(|path| {
            let contents = std::fs::read_to_string(path).ok()?;
            let payload: ItemFilePayload = serde_json::from_str(&contents).ok()?;
            let item_val = payload.item?;
            let mut item = Item::from_payload(&item_val, payload.raw.as_ref())?;

            // Resolve description label for translations
            if let Some(label_key) = Item::description_label(payload.raw.as_ref()) {
                if let Some(label) = labels.get(&label_key) {
                    for (locale, text) in &label.translations {
                        if locale != "en" {
                            item.translations.insert(locale.clone(), text.clone());
                        }
                    }
                }
            }

            Some(item)
        })
        .collect()
}

fn backfill_item_translations(
    _items: &mut [Item],
    _scunpacked: &Path,
    _labels: &HashMap<String, GameLabel>,
) {
    // Translations are already resolved during load_items via description labels.
    // This function is a hook for additional translation backfilling if needed.
}

fn load_vehicles(scunpacked: &Path) -> Vec<Vehicle> {
    let ships_dir = scunpacked.join("ships");
    if !ships_dir.is_dir() {
        warn!("Ships directory not found: {}", ships_dir.display());
        return Vec::new();
    }

    let pattern = ships_dir.join("*.json").to_string_lossy().to_string();
    let paths: Vec<PathBuf> = glob::glob(&pattern)
        .map(|paths| {
            paths
                .filter_map(Result::ok)
                .filter(|p| {
                    !p.file_name()
                        .and_then(|f| f.to_str())
                        .is_some_and(|f| f.ends_with("-raw.json"))
                })
                .collect()
        })
        .unwrap_or_default();

    paths
        .par_iter()
        .filter_map(|path| {
            let contents = std::fs::read_to_string(path).ok()?;
            let payload: VehicleFilePayload = serde_json::from_str(&contents).ok()?;
            Vehicle::from_payload(payload)
        })
        .collect()
}

fn load_starmap(scunpacked: &Path) -> Vec<StarmapLocation> {
    let path = scunpacked.join("starmap.json");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Could not read starmap.json: {e}");
            return Vec::new();
        }
    };

    let raw: Vec<serde_json::Value> = serde_json::from_str(&contents).unwrap_or_default();
    let mut locations: Vec<StarmapLocation> = raw.iter()
        .filter_map(StarmapLocation::from_payload)
        .collect();

    // Resolve system names via hierarchy
    resolve_starmap_hierarchy(&mut locations);

    locations
}

fn resolve_starmap_hierarchy(locations: &mut [StarmapLocation]) {
    // Build uuid -> index map
    let uuid_to_idx: HashMap<String, usize> = locations.iter().enumerate()
        .map(|(i, l)| (l.uuid.clone(), i))
        .collect();

    // Build solar system name lookup
    let solar_systems: HashMap<String, String> = locations.iter()
        .filter(|l| l.type_name == "SolarSystem")
        .map(|l| {
            let normalized = l.name.trim().to_lowercase()
                .trim_end_matches(" system")
                .to_string();
            (normalized, l.name.clone())
        })
        .collect();

    // Resolve system for each location by walking up the parent chain
    let parent_chains: Vec<(usize, Option<String>)> = locations.iter().enumerate()
        .map(|(i, _loc)| {
            let system = resolve_system_for_location(i, locations, &uuid_to_idx, &solar_systems);
            (i, system)
        })
        .collect();

    for (i, system) in parent_chains {
        locations[i].system = system;
    }
}

fn resolve_system_for_location(
    idx: usize,
    locations: &[StarmapLocation],
    uuid_to_idx: &HashMap<String, usize>,
    solar_systems: &HashMap<String, String>,
) -> Option<String> {
    let loc = &locations[idx];

    if loc.type_name == "SolarSystem" {
        return Some(loc.name.clone());
    }

    // Walk up parent chain
    let mut current_uuid = loc.parent_uuid.as_deref();
    let mut visited = std::collections::HashSet::new();

    while let Some(puuid) = current_uuid {
        if !visited.insert(puuid.to_string()) {
            break; // cycle
        }
        if let Some(&pidx) = uuid_to_idx.get(puuid) {
            let parent = &locations[pidx];
            if parent.type_name == "SolarSystem" {
                return Some(parent.name.clone());
            }
            current_uuid = parent.parent_uuid.as_deref();
        } else {
            break;
        }
    }

    // For stars, try matching by name to a solar system
    if loc.type_name == "Star" {
        let normalized = loc.name.trim().to_lowercase()
            .trim_end_matches(" system")
            .to_string();
        return solar_systems.get(&normalized).cloned();
    }

    None
}
