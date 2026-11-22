mod background_worker;
mod helper_functions;
mod init;
mod types;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use background_worker::background_metrics_updater;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use types::*;

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            interval_secs: Arc::clone(&self.interval_secs),
            smart_config: Arc::clone(&self.smart_config),
            cancel_token: Arc::clone(&self.cancel_token),
        }
    }
}

/// Helper to cancel the active worker and spawn a new one with updated settings
async fn restart_background_worker(state: &AppState, interval_secs: u64) {
    // 1. Cancel currently running loop
    {
        let cancel_guard = state.cancel_token.read().await;
        cancel_guard.cancel();
    }

    // 2. Replace cancellation token
    let new_token = CancellationToken::new();
    let worker_token = new_token.clone();
    {
        let mut cancel_write = state.cancel_token.write().await;
        *cancel_write = new_token;
    }

    // 3. Snapshot current SmartConfig and spawn worker
    let smart_cfg_snapshot = state.smart_config.read().await.clone();
    let metrics_clone = Arc::clone(&state.metrics);
    let duration = Duration::from_secs(interval_secs);

    tokio::spawn(async move {
        background_metrics_updater(metrics_clone, duration, smart_cfg_snapshot, worker_token).await;
    });
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
        return Json(JsonResponse {
            ok: false,
            message: None,
            error: Some("Polling Seconds Must Be Greater Than or Equals to 1.".into()),
        });
    }

    let is_already_set = {
        let interval_guard = state.interval_secs.read().await;
        *interval_guard == params.seconds
    };

    if is_already_set {
        return Json(JsonResponse {
            ok: true,
            message: Some("Already At the Specified Polling Rate".into()),
            error: None,
        });
    }

    // 1. Update fast polling duration
    {
        let mut interval_guard = state.interval_secs.write().await;
        *interval_guard = params.seconds;
    }

    // 2. Update SMART interval to n * 60 seconds (enforcing minimum 300s / 5 mins)
    let smart_secs = (params.seconds.saturating_mul(60)).max(300);
    {
        let mut smart_guard = state.smart_config.write().await;
        smart_guard.interval_secs = smart_secs;
    }

    // 3. Restart background worker
    restart_background_worker(&state, params.seconds).await;

    Json(JsonResponse {
        ok: true,
        message: Some(format!(
            "Interval updated to {}s (SMART interval: {}s). Background worker restarted.",
            params.seconds, smart_secs
        )),
        error: None,
    })
}

/// Optional endpoint to add/override extra smartctl flags per drive at runtime (e.g. USB enclosures)
async fn set_smart_device_args(
    State(state): State<AppState>,
    Json(payload): Json<SmartDeviceArgsPayload>,
) -> Json<JsonResponse> {
    if payload.device.trim().is_empty() {
        return Json(JsonResponse {
            ok: false,
            message: None,
            error: Some("Device path cannot be empty.".into()),
        });
    }

    {
        let mut smart_guard = state.smart_config.write().await;
        if payload.args.is_empty() {
            smart_guard.device_extra_args.remove(&payload.device);
        } else {
            smart_guard
                .device_extra_args
                .insert(payload.device.clone(), payload.args);
        }
    }

    // Clear last_updated timestamp so the restarted worker immediately probes the newly configured drive
    {
        let mut metrics_guard = state.metrics.write().await;
        metrics_guard.storage.smart_last_updated = None;
    }

    let current_interval = *state.interval_secs.read().await;
    restart_background_worker(&state, current_interval).await;

    Json(JsonResponse {
        ok: true,
        message: Some(format!(
            "Updated SMART args for {} and triggered fresh scan.",
            payload.device
        )),
        error: None,
    })
}

// --- 4. Main Setup ---

#[tokio::main]
async fn main() {
    let default_interval_secs: u64 = 5;
    let initial_token = CancellationToken::new();

    // Configure any known USB / RAID drive flags at startup (can also be updated via POST /smart-args)
    let initial_device_args: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // Example:
    // initial_device_args.insert(
    //     "/dev/sdb".to_string(),
    //     vec!["-d".into(), "sat".into(), "-n".into(), "standby".into()],
    // );

    let initial_smart_config = SmartConfig {
        interval_secs: (default_interval_secs * 60).max(300), // n * 60s, min 5 mins (300s)
        device_extra_args: initial_device_args,
    };

    let state = AppState {
        metrics: Arc::new(RwLock::new(MetricsSnapshot::default())),
        interval_secs: Arc::new(RwLock::new(default_interval_secs)),
        smart_config: Arc::new(RwLock::new(initial_smart_config.clone())),
        cancel_token: Arc::new(RwLock::new(initial_token.clone())),
    };

    // Spin up the initial loop process execution
    let metrics_clone = Arc::clone(&state.metrics);
    tokio::spawn(async move {
        background_metrics_updater(
            metrics_clone,
            Duration::from_secs(default_interval_secs),
            initial_smart_config,
            initial_token,
        )
        .await;
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build Axum router with chained paths
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/change-interval", get(change_interval))
        .route("/smart-args", post(set_smart_device_args))
        .with_state(state)
        .layer(cors);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Telemetry server ready on http://0.0.0.0:3000/metrics");

    axum::serve(listener, app).await.unwrap();
}
