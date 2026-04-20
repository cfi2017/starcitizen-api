use axum::{
    extract::{Path, State},
    http::Uri,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::query::QueryParams;
use crate::loader::GameData;
use crate::models::response::{paginate, PaginatedResponse};

type AppState = Arc<GameData>;

fn resource_type_to_response(rt: &crate::models::resource_type::ResourceType) -> Value {
    serde_json::json!({
        "uuid": rt.uuid,
        "key": rt.key,
        "name": rt.name,
        "description": rt.description,
        "refined_version_uuid": rt.refined_version_uuid,
        "validate_default_cargo_box": rt.validate_default_cargo_box,
        "has_default_cargo_containers": rt.has_default_cargo_containers,
        "box_sizes_scu": rt.box_sizes_scu,
    })
}

pub async fn list_resource_types(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let mut rts: Vec<_> = data.resource_types.iter().collect();

    // Filter: used=true means only show resource types referenced by blueprints
    if let Some(used) = params.filters.get("used") {
        let used = used == "true" || used == "1";
        if used {
            let used_uuids: std::collections::HashSet<&str> = data.blueprints.iter()
                .flat_map(|bp| bp.ingredient_resource_type_uuids.iter().map(|s| s.as_str()))
                .collect();
            rts.retain(|rt| used_uuids.contains(rt.uuid.as_str()));
        }
    }

    let responses: Vec<Value> = rts.iter().map(|rt| resource_type_to_response(rt)).collect();
    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

pub async fn resource_type_blueprints(
    State(data): State<AppState>,
    Path(uuid): Path<String>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let matching: Vec<Value> = data.blueprints.iter()
        .filter(|bp| bp.ingredient_resource_type_uuids.iter().any(|u| *u == uuid))
        .map(|bp| serde_json::json!({
            "uuid": bp.uuid,
            "key": bp.key,
            "output_name": bp.output_name,
        }))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&matching, params.page, params.per_page, &base_url))
}
