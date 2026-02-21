pub mod persona;
pub mod registry;

use anyhow::Result;
use axum::{
    extract::State,
    http::{Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use registry::{ConnectResult, PersonaRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    registry: Arc<Mutex<PersonaRegistry>>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub registry_personas: usize,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub model_id: String,
    #[serde(default = "default_persistent")]
    pub persistent: bool,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub persona_id: String,
    pub persona_name: String,
    pub model_id: String,
    pub first_name: String,
    pub last_name: String,
    pub birth_unix_ms: i64,
    pub persistent: bool,
    pub expires_at_unix_ms: Option<i64>,
    pub created: bool,
    pub system_prompt: String,
}

fn default_persistent() -> bool {
    true
}

type HttpResult<T> = std::result::Result<Json<T>, (StatusCode, String)>;

#[tokio::main]
async fn main() -> Result<()> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let app_state = AppState {
        registry: Arc::new(Mutex::new(PersonaRegistry::load_default()?)),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/agent/connect", post(agent_connect))
        .layer(cors)
        .with_state(app_state);

    let port = 9005;
    let addr = format!("0.0.0.0:{}", port);
    println!("LLM Identity Tool listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let registry_count = {
        let registry = state.registry.lock().await;
        registry.count()
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        registry_personas: registry_count,
    })
}

async fn agent_connect(
    State(state): State<AppState>,
    Json(payload): Json<ConnectRequest>,
) -> HttpResult<ConnectResponse> {
    let connected = {
        let mut registry = state.registry.lock().await;
        registry
            .connect_or_create(&payload.model_id, payload.persistent)
            .map_err(internal_server_error)?
    };

    Ok(Json(to_connect_response(connected)))
}

fn to_connect_response(connected: ConnectResult) -> ConnectResponse {
    ConnectResponse {
        persona_id: connected.persona.id,
        persona_name: connected.persona.name,
        system_prompt: connected.persona.system_prompt,
        model_id: connected.model_id,
        first_name: connected.first_name,
        last_name: connected.last_name,
        birth_unix_ms: connected.birth_unix_ms,
        persistent: connected.persistent,
        expires_at_unix_ms: connected.expires_at_unix_ms,
        created: connected.created,
    }
}

fn internal_server_error<E: std::fmt::Display>(error: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
