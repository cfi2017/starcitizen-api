use axum::{
    extract::{Path, State},
    http::Uri,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::query::{matches_search, QueryParams};
use crate::loader::GameData;
use crate::models::response::{paginate, single, PaginatedResponse};
use crate::models::starmap::StarmapLocation;

type AppState = Arc<GameData>;

fn location_to_response(loc: &StarmapLocation) -> Value {
    let amenities: Vec<Value> = loc.amenities.iter()
        .map(|a| serde_json::json!({
            "uuid": a.uuid,
            "name": a.name,
            "display_name": a.display_name,
        }))
        .collect();

    serde_json::json!({
        "uuid": loc.uuid,
        "name": loc.name,
        "description": loc.description,
        "type_name": loc.type_name,
        "size": loc.size,
        "is_scannable": loc.is_scannable,
        "block_travel": loc.block_travel,
        "parent_uuid": loc.parent_uuid,
        "system": loc.system,
        "amenities": amenities,
        "data": loc.data,
    })
}

pub async fn list_locations(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let mut locations: Vec<&StarmapLocation> = data.starmap_locations.iter().collect();

    // Apply filters
    for (key, value) in &params.filters {
        let value_lower = value.to_lowercase();
        locations.retain(|loc| match key.as_str() {
            "type" | "type_name" => loc.type_name.to_lowercase() == value_lower,
            "system" => loc.system.as_ref().is_some_and(|s| s.to_lowercase() == value_lower),
            "name" => matches_search(&loc.name, value),
            "amenity" => loc.amenities.iter().any(|a| {
                matches_search(&a.name, value)
                    || a.uuid == *value
                    || a.display_name.as_ref().is_some_and(|dn| matches_search(dn, value))
            }),
            _ => true,
        });
    }

    if let Some(ref search) = params.search {
        locations.retain(|loc| {
            matches_search(&loc.name, search)
                || loc.description.as_ref().is_some_and(|d| matches_search(d, search))
        });
    }

    let responses: Vec<Value> = locations.iter().map(|l| location_to_response(l)).collect();
    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

pub async fn get_location(
    State(data): State<AppState>,
    Path(identifier): Path<String>,
) -> Json<Value> {
    match data.starmap_locations_by_uuid.get(&identifier) {
        Some(&idx) => {
            let response = location_to_response(&data.starmap_locations[idx]);
            Json(serde_json::to_value(single(response)).unwrap())
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Location '{identifier}' not found"),
        })),
    }
}

pub async fn location_filters(State(data): State<AppState>) -> Json<Value> {
    let types: Vec<&str> = data.starmap_locations.iter()
        .map(|l| l.type_name.as_str())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let systems: Vec<&str> = data.starmap_locations.iter()
        .filter_map(|l| l.system.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Json(serde_json::json!({
        "type_name": types,
        "system": systems,
    }))
}
