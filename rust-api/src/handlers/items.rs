use axum::{
    extract::{Path, State},
    http::Uri,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;

use crate::handlers::query::{matches_search, sort_by_json_path, QueryParams};
use crate::loader::GameData;
use crate::models::item::Item;
use crate::models::response::{paginate, single, PaginatedResponse};

type AppState = Arc<GameData>;

/// Serialize an item for API response.
fn item_to_response(item: &Item, data: &GameData, locale: Option<&str>) -> Value {
    let manufacturer = item
        .manufacturer_uuid
        .as_ref()
        .and_then(|uuid| data.manufacturers_by_uuid.get(uuid))
        .map(|&idx| &data.manufacturers[idx]);

    let description = match locale {
        Some(loc) => item.translations.get(loc)
            .or_else(|| item.translations.get("en"))
            .map(|s| Value::String(s.clone()))
            .unwrap_or(Value::Null),
        None => {
            if item.translations.is_empty() {
                Value::Null
            } else {
                Value::Object(
                    item.translations
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                        .collect(),
                )
            }
        }
    };

    let entity_tag_map: Vec<Value> = item
        .entity_tags
        .iter()
        .filter_map(|uuid| {
            data.tags_by_uuid.get(uuid).map(|&idx| {
                let tag = &data.entity_tags[idx];
                serde_json::json!({
                    "uuid": tag.uuid,
                    "name": tag.name,
                })
            })
        })
        .collect();

    serde_json::json!({
        "uuid": item.uuid,
        "name": item.name,
        "class_name": item.class_name,
        "type": item.item_type,
        "sub_type": item.sub_type,
        "classification": item.classification,
        "size": item.size,
        "grade": item.grade,
        "class": item.class,
        "description": description,
        "manufacturer": manufacturer.map(|m| serde_json::json!({
            "uuid": m.uuid,
            "name": m.name,
            "code": m.code,
        })),
        "entity_tags": item.entity_tags,
        "entity_tag_map": entity_tag_map,
        "data": item.data,
    })
}

/// GET /items
pub async fn list_items(
    State(data): State<AppState>,
    uri: Uri,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));
    let items = filter_items(&data, &params, None);

    let responses: Vec<Value> = items
        .iter()
        .map(|item| item_to_response(item, &data, params.locale.as_deref()))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

/// GET /items/:identifier
pub async fn get_item(
    State(data): State<AppState>,
    Path(identifier): Path<String>,
    uri: Uri,
) -> Json<Value> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let item = data
        .items_by_uuid
        .get(&identifier)
        .or_else(|| data.items_by_name.get(&identifier.to_lowercase()))
        .map(|&idx| &data.items[idx]);

    match item {
        Some(item) => {
            let response = item_to_response(item, &data, params.locale.as_deref());
            Json(serde_json::to_value(single(response)).unwrap())
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("Item '{identifier}' not found"),
        })),
    }
}

/// POST /items/search
pub async fn search_items(
    State(data): State<AppState>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Json<PaginatedResponse<Value>> {
    let mut params = QueryParams::from_query(uri.query().unwrap_or(""));

    if let Some(query) = body.get("query").and_then(|v| v.as_str()) {
        params.search = Some(query.to_string());
    }

    let items = filter_items(&data, &params, None);
    let responses: Vec<Value> = items
        .iter()
        .map(|item| item_to_response(item, &data, params.locale.as_deref()))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

/// GET /items/filters
pub async fn item_filters(State(data): State<AppState>) -> Json<Value> {
    let types: Vec<&str> = data.items.iter()
        .filter_map(|i| i.item_type.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let sub_types: Vec<&str> = data.items.iter()
        .filter_map(|i| i.sub_type.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let classifications: Vec<&str> = data.items.iter()
        .filter_map(|i| i.classification.as_deref())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Json(serde_json::json!({
        "type": types,
        "sub_type": sub_types,
        "classification": classifications,
    }))
}

/// Category-filtered item endpoints (weapons, armor, clothes, food, etc.)
pub async fn list_items_by_category(
    State(data): State<AppState>,
    uri: Uri,
    category: &str,
) -> Json<PaginatedResponse<Value>> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));
    let items = filter_items(&data, &params, Some(category));

    let responses: Vec<Value> = items
        .iter()
        .map(|item| item_to_response(item, &data, params.locale.as_deref()))
        .collect();

    let base_url = uri.path().to_string();
    Json(paginate(&responses, params.page, params.per_page, &base_url))
}

pub async fn get_item_by_category(
    State(data): State<AppState>,
    Path(identifier): Path<String>,
    uri: Uri,
    category: &str,
) -> Json<Value> {
    let params = QueryParams::from_query(uri.query().unwrap_or(""));

    let item = data.items.iter().find(|i| {
        (i.uuid == identifier || i.name.to_lowercase() == identifier.to_lowercase())
            && matches_category(i, category)
    });

    match item {
        Some(item) => {
            let response = item_to_response(item, &data, params.locale.as_deref());
            Json(serde_json::to_value(single(response)).unwrap())
        }
        None => Json(serde_json::json!({
            "error": "Not found",
            "message": format!("{category} '{identifier}' not found"),
        })),
    }
}

// Wrappers for category endpoints
pub async fn list_weapons(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "weapons").await
}
pub async fn get_weapon(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "weapons").await
}
pub async fn list_weapon_attachments(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "weapon-attachments").await
}
pub async fn get_weapon_attachment(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "weapon-attachments").await
}
pub async fn list_clothes(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "clothes").await
}
pub async fn get_cloth(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "clothes").await
}
pub async fn list_armor(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "armor").await
}
pub async fn get_armor(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "armor").await
}
pub async fn list_food(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "food").await
}
pub async fn get_food(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "food").await
}
pub async fn list_vehicle_weapons(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "vehicle-weapons").await
}
pub async fn get_vehicle_weapon(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "vehicle-weapons").await
}
pub async fn list_vehicle_items(s: State<AppState>, u: Uri) -> Json<PaginatedResponse<Value>> {
    list_items_by_category(s, u, "vehicle-items").await
}
pub async fn get_vehicle_item(s: State<AppState>, p: Path<String>, u: Uri) -> Json<Value> {
    get_item_by_category(s, p, u, "vehicle-items").await
}

fn filter_items<'a>(
    data: &'a GameData,
    params: &QueryParams,
    category: Option<&str>,
) -> Vec<&'a Item> {
    let mut items: Vec<&Item> = data.items.iter().collect();

    // Category filter
    if let Some(cat) = category {
        items.retain(|item| matches_category(item, cat));
    }

    // Apply filters
    for (key, value) in &params.filters {
        let value_lower = value.to_lowercase();
        items.retain(|item| match key.as_str() {
            "type" => item.item_type.as_ref().is_some_and(|t| t.to_lowercase() == value_lower),
            "sub_type" => item.sub_type.as_ref().is_some_and(|t| t.to_lowercase() == value_lower),
            "manufacturer" => item.manufacturer_uuid.as_ref().is_some_and(|uuid| {
                // Match by UUID or by name/code via lookup
                if uuid.to_lowercase() == value_lower {
                    return true;
                }
                if let Some(&idx) = data.manufacturers_by_uuid.get(uuid.as_str()) {
                    let m = &data.manufacturers[idx];
                    return m.name.to_lowercase() == value_lower
                        || m.code.to_lowercase() == value_lower;
                }
                false
            }),
            "class_name" => item.class_name.as_ref().is_some_and(|c| c.to_lowercase() == value_lower),
            "name" => matches_search(&item.name, value),
            "classification" => item.classification.as_ref().is_some_and(|c| c.to_lowercase() == value_lower),
            "size" => item.size.map(|s| s.to_string() == *value).unwrap_or(false),
            "grade" => item.grade.map(|g| g.to_string() == *value).unwrap_or(false),
            "class" => item.class.as_ref().is_some_and(|c| c.to_lowercase() == value_lower),
            _ => true,
        });
    }

    // Search
    if let Some(ref query) = params.search {
        let query = query.replace('_', " ");
        items.retain(|item| {
            matches_search(&item.name, &query)
                || item.class_name.as_ref().is_some_and(|c| matches_search(c, &query))
        });
    }

    // Sort
    if let Some(ref sort_spec) = params.sort {
        let indices = sort_by_json_path(&items, sort_spec, |item| &item.data);
        items = indices.into_iter().map(|i| items[i]).collect();
    }

    items
}

fn matches_category(item: &Item, category: &str) -> bool {
    let t = item.item_type.as_deref().unwrap_or("").to_lowercase();
    let st = item.sub_type.as_deref().unwrap_or("").to_lowercase();
    let cl = item.classification.as_deref().unwrap_or("").to_lowercase();

    match category {
        "weapons" => t == "weapon" || cl.contains("weapon"),
        "weapon-attachments" => t == "weaponattachment" || st.contains("attachment"),
        "clothes" => t == "char_clothing",
        "armor" => t == "char_armor" || cl.contains("armor"),
        "food" => t == "consumable" && (st == "food" || st == "drink"),
        "vehicle-weapons" => t == "vehicleweapon" || (t == "weapon" && cl.contains("vehicle")),
        "vehicle-items" => cl.contains("vehicle") && t != "vehicleweapon",
        _ => true,
    }
}
