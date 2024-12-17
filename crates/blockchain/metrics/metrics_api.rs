use axum::{routing::get, Router};
use prometheus::{Encoder, IntCounter, Registry, TextEncoder};
use std::sync::{Arc, LazyLock, Mutex};

use crate::MetricsApiError;

pub static TRANSACTION_COUNTER: LazyLock<Arc<Mutex<IntCounter>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(
        IntCounter::new(
            "transactions_counter",
            "keeps track of the executed transactions",
        )
        .unwrap(),
    ))
});

pub async fn start_prometheus_metrics_api(port: String) -> Result<(), MetricsApiError> {
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/health", get("Service Up"));

    // Start the axum app
    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_metrics() -> String {
    let r = Registry::new();
    let tx_counter = TRANSACTION_COUNTER.clone();

    let tx_counter_lock = match tx_counter.lock() {
        Ok(lock) => lock,
        Err(e) => {
            tracing::error!("Failed to lock mutex: {e}");
            return String::new();
        }
    };

    if r.register(Box::new(tx_counter_lock.clone())).is_err() {
        tracing::error!("Failed to register metric");
        return String::new();
    }

    let encoder = TextEncoder::new();
    let metric_families = r.gather();

    let mut buffer = Vec::new();
    if encoder.encode(&metric_families, &mut buffer).is_err() {
        tracing::error!("Failed to encode metrics");
        return String::new();
    }

    String::from_utf8(buffer).unwrap_or_else(|e| {
        tracing::error!("Failed to convert buffer to String: {e}");
        String::new()
    })
}
