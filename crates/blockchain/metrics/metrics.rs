use std::sync::{Arc, LazyLock, Mutex};

use axum::{routing::get, Router};
use prometheus::{Encoder, IntCounter, Registry, TextEncoder};

pub static TRANSACTION_COUNTER: LazyLock<Arc<Mutex<IntCounter>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(
        IntCounter::new(
            "transactions_counter",
            "keeps track of the executed transactions",
        )
        .unwrap(),
    ))
});

pub async fn start_prometheus_metrics_api(port: String) {
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/health", get("Service Up"));

    // Start the axum app
    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_metrics() -> String {
    let r = Registry::new();
    let tx_counter = TRANSACTION_COUNTER.clone();
    r.register(Box::new(tx_counter.lock().unwrap().clone()))
        .unwrap();

    let encoder = TextEncoder::new();
    let metric_families = r.gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let str = String::from_utf8(buffer).unwrap();
    tracing::info!("{str}");

    str
}
