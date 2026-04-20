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

type AppState = Arc<GameData>;

fn manufacturer_to_response(m: &crate::models::manufacturer::Manufacturer) -> Value {
    serde_json::json!({
        "uuid": m.uuid,
        "name": m.name,
        "code": m.code,
    })
}

pub async fn list_manufacturers(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let mut mfrs: Vec<_> = data.manufacturers.iter().collect();

    if let Some(name_filter) = params.filters.get("name") {
        mfrs.retain(|m| matches_search(&m.name, name_filter));
    }

    if let Some(ref search) = params.search {
        mfrs.retain(|m| matches_search(&m.name, search) || matches_search(&m.code, search));
    }

    let responses: Vec<Value> = mfrs.iter().map(|m| manufacturer_to_response(m)).collect();
    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

pub async fn get_manufacturer(
    State(data): State<AppState>,
    Path(identifier): Path<String>,
) -> Json<Value> {
    let id_lower = identifier.to_lowercase();

    let mfr = data.manufacturers.iter().find(|m| {
        m.uuid == identifier || m.name.to_lowercase() == id_lower || m.code.to_lowercase() == id_lower
    });

    match mfr {
        Some(m) => Json(serde_json::to_value(single(manufacturer_to_response(m))).unwrap()),
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Manufacturer '{identifier}' not found"),
        })),
    }
}

pub async fn search_manufacturers(
    State(data): State<AppState>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<PaginatedResponse<Value>> {
    let mut params = QueryParams::from_query(uri.query().unwrap_or(""));
    if let Some(query) = body.get("query").and_then(|v| v.as_str()) {
        params.search = Some(query.to_string());
    }

    let mut mfrs: Vec<_> = data.manufacturers.iter().collect();

    if let Some(ref search) = params.search {
        mfrs.retain(|m| matches_search(&m.name, search) || matches_search(&m.code, search));
    }

    let responses: Vec<Value> = mfrs.iter().map(|m| manufacturer_to_response(m)).collect();
    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}
