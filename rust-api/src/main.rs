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
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    // Initialize tracing FIRST so panics during loading are visible.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "starcitizen_api=info,tower_http=info".into()),
        )
        // Use JSON format in production for structured logging
        .json()
        .flatten_event(true)
        .with_current_span(false)
        .init();

    // Install a panic hook that logs via tracing before aborting.
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };

        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));

        eprintln!("PANIC: {payload} at {}", location.as_deref().unwrap_or("unknown"));
        // Also emit via tracing in case stderr is swallowed
        tracing::error!(
            panic = true,
            message = %payload,
            location = location.as_deref().unwrap_or("unknown"),
            "Application panicked"
        );
    }));

    info!(version = env!("CARGO_PKG_VERSION"), "starcitizen-rust-api starting");

    let data_dir = env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/data"));

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    info!(data_dir = %data_dir.display(), "Resolved data directory");

    // Check data directory exists
    if !data_dir.exists() {
        error!(path = %data_dir.display(), "DATA_DIR does not exist");
        std::process::exit(1);
    }

    if !data_dir.is_dir() {
        error!(path = %data_dir.display(), "DATA_DIR is not a directory");
        std::process::exit(1);
    }

    // List contents for debugging
    match std::fs::read_dir(&data_dir) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            info!(contents = ?names, "DATA_DIR contents");
        }
        Err(e) => {
            error!(error = %e, path = %data_dir.display(), "Failed to read DATA_DIR");
            std::process::exit(1);
        }
    }

    // Check scunpacked-data subdirectory
    let scunpacked = data_dir.join("scunpacked-data");
    if !scunpacked.is_dir() {
        warn!(path = %scunpacked.display(), "scunpacked-data directory not found - service will start with empty data");
    }

    info!("Loading game data...");
    let data = Arc::new(loader::GameData::load(&data_dir));
    info!(
        items = data.items.len(),
        vehicles = data.vehicles.len(),
        blueprints = data.blueprints.len(),
        manufacturers = data.manufacturers.len(),
        resource_types = data.resource_types.len(),
        starmap_locations = data.starmap_locations.len(),
        entity_tags = data.entity_tags.len(),
        labels = data.labels.len(),
        "Data loading complete"
    );

    let app = build_router(data);

    info!(addr = %bind_addr, "Binding listener");
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, addr = %bind_addr, "Failed to bind listener");
            std::process::exit(1);
        }
    };

    info!(addr = %bind_addr, "Server ready, accepting connections");

    let shutdown = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received SIGINT, shutting down gracefully");
            }
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully");
            }
        }
    };

    match axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        Ok(()) => {
            info!("Server shut down gracefully");
        }
        Err(e) => {
            error!(error = %e, "Server error");
            std::process::exit(1);
        }
    }
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
