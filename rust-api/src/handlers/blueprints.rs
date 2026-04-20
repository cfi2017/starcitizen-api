use axum::{
    extract::{Path, State},
    http::Uri,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::query::{matches_search, QueryParams};
use crate::loader::GameData;
use crate::models::blueprint::Blueprint;
use crate::models::response::{paginate, single, PaginatedResponse};

type AppState = Arc<GameData>;

fn blueprint_to_response(bp: &Blueprint) -> Value {
    serde_json::json!({
        "uuid": bp.uuid,
        "key": bp.key,
        "category_uuid": bp.category_uuid,
        "output_item_uuid": bp.output_item_uuid,
        "output_name": bp.output_name,
        "output_class": bp.output_class,
        "craft_time_seconds": bp.craft_time_seconds,
        "is_available_by_default": bp.is_available_by_default,
        "ingredient_count": bp.ingredient_resource_type_uuids.len(),
        "data": bp.data,
    })
}

pub async fn list_blueprints(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let mut blueprints: Vec<&Blueprint> = data.blueprints.iter().collect();

    // Apply filters
    for (key, value) in &params.filters {
        let value_lower = value.to_lowercase();
        blueprints.retain(|bp| match key.as_str() {
            "query" => {
                bp.key.to_lowercase().contains(&value_lower)
                    || bp.output_name.as_ref().is_some_and(|n| n.to_lowercase().contains(&value_lower))
            }
            "output.uuid" => bp.output_item_uuid.as_ref().is_some_and(|u| {
                value.split(',').any(|v| v.trim() == u.as_str())
            }),
            "output.name" => bp.output_name.as_ref().is_some_and(|n| matches_search(n, value)),
            "output.class" => bp.output_class.as_ref().is_some_and(|c| matches_search(c, value)),
            "default" => bp.is_available_by_default == (value_lower == "true" || value_lower == "1"),
            "ingredient" => bp.ingredient_resource_type_uuids.iter().any(|uuid| {
                if let Some(&idx) = data.resource_types_by_uuid.get(uuid.as_str()) {
                    let rt = &data.resource_types[idx];
                    matches_search(&rt.name, value) || matches_search(&rt.key, value)
                } else {
                    false
                }
            }),
            _ => true,
        });
    }

    if let Some(ref search) = params.search {
        blueprints.retain(|bp| {
            bp.key.to_lowercase().contains(&search.to_lowercase())
                || bp.output_name.as_ref().is_some_and(|n| matches_search(n, search))
        });
    }

    let responses: Vec<Value> = blueprints.iter().map(|bp| blueprint_to_response(bp)).collect();
    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

pub async fn get_blueprint(
    State(data): State<AppState>,
    Path(uuid): Path<String>,
) -> Json<Value> {
    match data.blueprints_by_uuid.get(&uuid) {
        Some(&idx) => {
            let response = blueprint_to_response(&data.blueprints[idx]);
            Json(serde_json::to_value(single(response)).unwrap())
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Blueprint '{uuid}' not found"),
        })),
    }
}
