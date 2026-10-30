mod background_worker;
mod init;
mod types;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use background_worker::background_metrics_updater;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use types::*;

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            interval_secs: Arc::clone(&self.interval_secs),
            cancel_token: Arc::clone(&self.cancel_token),
        }
    }
}

// --- The Axum Handlers ---

async fn get_metrics(State(state): State<AppState>) -> Json<MetricsSnapshot> {
    let read_guard = state.metrics.read().await;
    let data = read_guard.clone();
    drop(read_guard);
    Json(data)
}

async fn change_interval(
    State(state): State<AppState>,
    Query(params): Query<IntervalQuery>,
) -> Json<JsonResponse> {
    if params.seconds == 0 {
        let res: JsonResponse = JsonResponse {
            ok: true,
            message: None,
            error: Some("Polling Seconds Must Be Greater Than or Equals to 1.".into()),
        };
        return Json(res);
    }

    // 1. Update global configuration duration value
    let mut interval_guard = state.interval_secs.write().await;
    *interval_guard = params.seconds;

    // 2. Cancel the currently running loop execution
    let cancel_guard = state.cancel_token.read().await;
    cancel_guard.cancel();
    drop(cancel_guard);

    // 3. Create a brand-new token replacement for the next worker
    let new_token = CancellationToken::new();
    let worker_token = new_token.clone();

    let mut cancel_write = state.cancel_token.write().await;
    *cancel_write = new_token;
    drop(cancel_write);

    // 4. Respawn background process execution with target duration parameters
    let metrics_clone = Arc::clone(&state.metrics);
    let duration = Duration::from_secs(params.seconds);

    tokio::spawn(async move {
        background_metrics_updater(metrics_clone, duration, worker_token).await;
    });

    let res: JsonResponse = JsonResponse {
        ok: true,
        message: Some(format!(
            "Interval successfully updated to {} seconds! Background worker restarted.",
            params.seconds
        )),
        error: None,
    };
    Json(res)
}

// --- 4. Main Setup ---

#[tokio::main]
async fn main() {
    let default_interval_secs = 5;
    let initial_token = CancellationToken::new();

    let state = AppState {
        metrics: Arc::new(RwLock::new(MetricsSnapshot::default())),
        interval_secs: Arc::new(RwLock::new(default_interval_secs)),
        cancel_token: Arc::new(RwLock::new(initial_token.clone())),
    };

    // Spin up the initial loop process execution
    let metrics_clone = Arc::clone(&state.metrics);
    tokio::spawn(async move {
        background_metrics_updater(
            metrics_clone,
            Duration::from_secs(default_interval_secs),
            initial_token,
        )
        .await;
    });

    // Build Axum router with chained paths
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/change-interval", get(change_interval))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 High-performance telemetry server on http://0.0.0.0:3000/metrics");
    println!("🔄 Background updater running every 5 seconds...");

    axum::serve(listener, app).await.unwrap();
}
