mod handlers;
mod loader;
mod models;

use axum::{
    Router,
    routing::{get, post},
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "starcitizen_api=info,tower_http=info".into()),
        )
        .init();

    let data_dir = env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/data"));

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    info!("Loading game data from {}", data_dir.display());
    let data = Arc::new(loader::GameData::load(&data_dir));
    info!("Data loaded: {}", data.memory_report());

    let app = build_router(data);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind");

    info!("Listening on {bind_addr}");
    axum::serve(listener, app).await.expect("Server error");
}

fn build_router(data: Arc<loader::GameData>) -> Router {
    // Game data endpoints (versioned, matches Laravel /api/v3/ and unversioned)
    let game_routes = Router::new()
        // Items
        .route("/items", get(handlers::items::list_items))
        .route("/items/filters", get(handlers::items::item_filters))
        .route("/items/search", post(handlers::items::search_items))
        .route("/items/{identifier}", get(handlers::items::get_item))
        // Category shortcuts
        .route("/weapons", get(handlers::items::list_weapons))
        .route("/weapons/{identifier}", get(handlers::items::get_weapon))
        .route("/weapon-attachments", get(handlers::items::list_weapon_attachments))
        .route("/weapon-attachments/{identifier}", get(handlers::items::get_weapon_attachment))
        .route("/clothes", get(handlers::items::list_clothes))
        .route("/clothes/{identifier}", get(handlers::items::get_cloth))
        .route("/armor", get(handlers::items::list_armor))
        .route("/armor/{identifier}", get(handlers::items::get_armor))
        .route("/food", get(handlers::items::list_food))
        .route("/food/{identifier}", get(handlers::items::get_food))
        .route("/vehicle-weapons", get(handlers::items::list_vehicle_weapons))
        .route("/vehicle-weapons/{identifier}", get(handlers::items::get_vehicle_weapon))
        .route("/vehicle-items", get(handlers::items::list_vehicle_items))
        .route("/vehicle-items/{identifier}", get(handlers::items::get_vehicle_item))
        // Vehicles
        .route("/vehicles", get(handlers::vehicles::list_vehicles))
        .route("/vehicles/filters", get(handlers::vehicles::vehicle_filters))
        .route("/vehicles/search", post(handlers::vehicles::search_vehicles))
        .route("/vehicles/{identifier}", get(handlers::vehicles::get_vehicle))
        .route("/ground-vehicles", get(handlers::vehicles::list_ground_vehicles))
        .route("/ground-vehicles/{identifier}", get(handlers::vehicles::get_ground_vehicle))
        .route("/gravlev-vehicles", get(handlers::vehicles::list_gravlev_vehicles))
        .route("/gravlev-vehicles/{identifier}", get(handlers::vehicles::get_gravlev_vehicle))
        // Blueprints
        .route("/blueprints", get(handlers::blueprints::list_blueprints))
        .route("/blueprints/{uuid}", get(handlers::blueprints::get_blueprint))
        // Manufacturers
        .route("/manufacturers", get(handlers::manufacturers::list_manufacturers))
        .route("/manufacturers/search", post(handlers::manufacturers::search_manufacturers))
        .route("/manufacturers/{identifier}", get(handlers::manufacturers::get_manufacturer))
        // Resource types
        .route("/resource-types", get(handlers::resource_types::list_resource_types))
        .route("/resource-types/{uuid}/blueprints", get(handlers::resource_types::resource_type_blueprints))
        // Starmap locations
        .route("/locations", get(handlers::starmap::list_locations))
        .route("/locations/filters", get(handlers::starmap::location_filters))
        .route("/locations/{identifier}", get(handlers::starmap::get_location));

    // Health endpoint
    let health = Router::new().route("/up", get(|| async { "OK" }));

    Router::new()
        .nest("/api", game_routes.clone())
        .nest("/api/v3", game_routes)
        .merge(health)
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(data)
}
