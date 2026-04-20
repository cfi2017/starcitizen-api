use axum::{
    extract::{Path, State},
    http::Uri,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::query::{matches_search, sort_by_json_path, QueryParams};
use crate::loader::GameData;
use crate::models::response::{paginate, single, PaginatedResponse};
use crate::models::vehicle::Vehicle;

type AppState = Arc<GameData>;

fn vehicle_to_response(vehicle: &Vehicle, data: &GameData, _locale: Option<&str>) -> Value {
    let manufacturer = vehicle
        .manufacturer_uuid
        .as_ref()
        .and_then(|uuid| data.manufacturers_by_uuid.get(uuid))
        .map(|&idx| &data.manufacturers[idx]);

    serde_json::json!({
        "uuid": vehicle.uuid,
        "name": vehicle.name,
        "display_name": vehicle.display_name,
        "class_name": vehicle.class_name,
        "career": vehicle.career,
        "role": vehicle.role,
        "is_vehicle": vehicle.is_vehicle,
        "is_gravlev": vehicle.is_gravlev,
        "is_spaceship": vehicle.is_spaceship,
        "size": vehicle.size,
        "manufacturer": manufacturer.map(|m| serde_json::json!({
            "uuid": m.uuid,
            "name": m.name,
            "code": m.code,
        })),
        "data": vehicle.data,
    })
}

/// GET /vehicles
pub async fn list_vehicles(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));
    let vehicles = filter_vehicles(&data, &params, None);

    let responses: Vec<Value> = vehicles
        .iter()
        .map(|v| vehicle_to_response(v, &data, params.locale.as_deref()))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

/// GET /vehicles/:identifier
pub async fn get_vehicle(
    State(data): State<AppState>,
    Path(identifier): Path<String>,
    uri: Uri,
) -> Json<Value> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));
    let id_lower = identifier.to_lowercase();

    let vehicle = data.vehicles.iter().find(|v| {
        v.uuid == identifier
            || v.name.as_ref().is_some_and(|n| n.to_lowercase() == id_lower)
            || v.display_name.as_ref().is_some_and(|n| n.to_lowercase() == id_lower)
    });

    match vehicle {
        Some(v) => {
            let response = vehicle_to_response(v, &data, params.locale.as_deref());
            Json(serde_json::to_value(single(response)).unwrap())
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Vehicle '{identifier}' not found"),
        })),
    }
}

/// POST /vehicles/search
pub async fn search_vehicles(
    State(data): State<AppState>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<PaginatedResponse<Value>> {
    let mut params = QueryParams::from_query(uri.query().unwrap_or(""));
    if let Some(query) = body.get("query").and_then(|v| v.as_str()) {
        params.search = Some(query.to_string());
    }

    let vehicles = filter_vehicles(&data, &params, None);
    let responses: Vec<Value> = vehicles
        .iter()
        .map(|v| vehicle_to_response(v, &data, params.locale.as_deref()))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

/// GET /vehicles/filters
pub async fn vehicle_filters(State(data): State<AppState>) -> Json<Value> {
    let careers: Vec<&str> = data.vehicles.iter()
        .filter_map(|v| v.career.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let roles: Vec<&str> = data.vehicles.iter()
        .filter_map(|v| v.role.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Json(serde_json::json!({
        "career": careers,
        "role": roles,
    }))
}

// Ground vehicles
pub async fn list_ground_vehicles(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(u.query().unwrap_or(""));
    let vehicles = filter_vehicles(&s, &params, Some("ground"));
    let responses: Vec<Value> = vehicles.iter()
        .map(|v| vehicle_to_response(v, &s, params.locale.as_deref()))
        .collect();
    Json(paginate(&responses, params.page, params.per_page, u.path()))
}

pub async fn get_ground_vehicle(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    let params = QueryParams::from_query(u.query().unwrap_or(""));
    let id_lower = p.to_lowercase();
    let vehicle = s.vehicles.iter().find(|v| {
        v.is_vehicle && !v.is_gravlev
            && (v.uuid == *p
                || v.name.as_ref().is_some_and(|n| n.to_lowercase() == id_lower))
    });
    match vehicle {
        Some(v) => Json(serde_json::to_value(single(vehicle_to_response(v, &s, params.locale.as_deref()))).unwrap()),
        None => Json(serde_json::json!({"error": "Not found"})),
    }
}

// Gravlev vehicles
pub async fn list_gravlev_vehicles(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(u.query().unwrap_or(""));
    let vehicles = filter_vehicles(&s, &params, Some("gravlev"));
    let responses: Vec<Value> = vehicles.iter()
        .map(|v| vehicle_to_response(v, &s, params.locale.as_deref()))
        .collect();
    Json(paginate(&responses, params.page, params.per_page, u.path()))
}

pub async fn get_gravlev_vehicle(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    let params = QueryParams::from_query(u.query().unwrap_or(""));
    let id_lower = p.to_lowercase();
    let vehicle = s.vehicles.iter().find(|v| {
        v.is_gravlev
            && (v.uuid == *p
                || v.name.as_ref().is_some_and(|n| n.to_lowercase() == id_lower))
    });
    match vehicle {
        Some(v) => Json(serde_json::to_value(single(vehicle_to_response(v, &s, params.locale.as_deref()))).unwrap()),
        None => Json(serde_json::json!({"error": "Not found"})),
    }
}

fn filter_vehicles<'a>(
    data: &'a GameData,
    params: &QueryParams,
    vehicle_type: Option<&str>,
) -> Vec<&'a Vehicle> {
    let mut vehicles: Vec<&Vehicle> = data.vehicles.iter().collect();

    // Type filter
    match vehicle_type {
        Some("ground") => vehicles.retain(|v| v.is_vehicle && !v.is_gravlev),
        Some("gravlev") => vehicles.retain(|v| v.is_gravlev),
        Some("spaceship") => vehicles.retain(|v| v.is_spaceship),
        _ => {}
    }

    // Apply filters
    for (key, value) in &params.filters {
        let value_lower = value.to_lowercase();
        vehicles.retain(|v| match key.as_str() {
            "manufacturer" | "manufacturer.name" => v.manufacturer_uuid.as_ref().is_some_and(|uuid| {
                if let Some(&idx) = data.manufacturers_by_uuid.get(uuid.as_str()) {
                    let m = &data.manufacturers[idx];
                    m.name.to_lowercase() == value_lower || m.code.to_lowercase() == value_lower
                } else {
                    false
                }
            }),
            "class_name" => v.class_name.as_ref().is_some_and(|c| c.to_lowercase() == value_lower),
            "name" => v.name.as_ref().is_some_and(|n| matches_search(n, value)),
            "career" => v.career.as_ref().is_some_and(|c| c.to_lowercase() == value_lower),
            "role" => v.role.as_ref().is_some_and(|r| r.to_lowercase() == value_lower),
            "is_vehicle" => v.is_vehicle == (value_lower == "true" || value_lower == "1"),
            "is_gravlev" => v.is_gravlev == (value_lower == "true" || value_lower == "1"),
            "is_spaceship" => v.is_spaceship == (value_lower == "true" || value_lower == "1"),
            _ => true,
        });
    }

    // Search
    if let Some(ref query) = params.search {
        let query = query.replace('_', " ");
        vehicles.retain(|v| {
            v.name.as_ref().is_some_and(|n| matches_search(n, &query))
                || v.display_name.as_ref().is_some_and(|n| matches_search(n, &query))
                || v.class_name.as_ref().is_some_and(|c| matches_search(c, &query))
        });
    }

    // Sort
    if let Some(ref sort_spec) = params.sort {
        let indices = sort_by_json_path(&vehicles, sort_spec, |v| &v.data);
        vehicles = indices.into_iter().map(|i| vehicles[i]).collect();
    }

    vehicles
}
