use axum::{routing::get, Router};

#[cfg(feature = "l2")]
use crate::metrics_l2::METRICS_L2;

use crate::{metrics_transactions::METRICS_TX, MetricsApiError};

pub async fn start_prometheus_metrics_api(port: String) -> Result<(), MetricsApiError> {
    let app = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/health", get("Service Up"));

    // Start the axum app
    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[allow(unused_mut)]
async fn get_metrics() -> String {
    let mut ret_string = METRICS_TX.gather_metrics();
    #[cfg(feature = "l2")]
    {
        ret_string.push('\n');
        ret_string.push_str(&METRICS_L2.gather_metrics())
    }

    ret_string
}
