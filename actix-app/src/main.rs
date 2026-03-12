use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, get, post, web};
use base64::prelude::*;
use std::collections::HashMap;

use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Histogram},
    trace::TracerProvider,
};
use opentelemetry_otlp::{
    LogExporter, MetricExporter, SpanExporter, WithExportConfig, WithHttpConfig,
};
use opentelemetry_sdk::{
    Resource,
    logs::SdkLoggerProvider,
    metrics::{PeriodicReader, SdkMeterProvider},
    trace::{Sampler, SdkTracerProvider},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, instrument, warn};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct OtelConfig {
    http_endpoint: String,
    org_id: String,
    stream: String,
    auth: String,
}

impl OtelConfig {
    fn from_env() -> Self {
        let http_endpoint = std::env::var("OPENOBSERVE_HTTP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:5080".into());
        let org_id = std::env::var("OPENOBSERVE_ORG").unwrap_or_else(|_| "default".into());
        let stream = std::env::var("OPENOBSERVE_STREAM").unwrap_or_else(|_| "actix_demo".into());
        let user = std::env::var("OPENOBSERVE_USER").unwrap_or_else(|_| "root@example.com".into());
        let pass = std::env::var("OPENOBSERVE_PASS").unwrap_or_else(|_| "Complexpass#123".into());
        let auth = format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{}:{}", user, pass))
        );
        OtelConfig {
            http_endpoint,
            org_id,
            stream,
            auth,
        }
    }
}

// ─── Metrics State ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AppMetrics {
    http_requests_total: Counter<u64>,
    http_errors_total: Counter<u64>,
    http_duration_ms: Histogram<f64>,
    stack_errors_total: Counter<u64>,
}

impl AppMetrics {
    fn new() -> Self {
        let meter = global::meter("actix-openobserve");
        AppMetrics {
            http_requests_total: meter
                .u64_counter("http_requests_total")
                .with_description("Total HTTP requests")
                .build(),
            http_errors_total: meter
                .u64_counter("http_errors_total")
                .with_description("Total HTTP errors (4xx/5xx)")
                .build(),
            http_duration_ms: meter
                .f64_histogram("http_request_duration_ms")
                .with_description("HTTP request duration in milliseconds")
                .build(),
            stack_errors_total: meter
                .u64_counter("stack_errors_total")
                .with_description("Total stack / panic errors captured")
                .build(),
        }
    }
}

// ─── Models ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Deserialize, Serialize, Debug)]
struct EchoPayload {
    message: String,
    #[serde(default)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
struct EchoResponse {
    request_id: String,
    echoed: EchoPayload,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

#[get("/health")]
#[instrument(name = "health_check")]
async fn health(metrics: web::Data<AppMetrics>) -> impl Responder {
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/health"),
            KeyValue::new("method", "GET"),
        ],
    );
    info!(endpoint = "/health", "Health check OK");
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[get("/metrics-demo")]
#[instrument(name = "metrics_demo")]
async fn metrics_demo(metrics: web::Data<AppMetrics>) -> impl Responder {
    let start = std::time::Instant::now();
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/metrics-demo"),
            KeyValue::new("method", "GET"),
        ],
    );
    tokio::time::sleep(Duration::from_millis(rand::random::<u8>() as u64 + 10)).await;
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    metrics
        .http_duration_ms
        .record(elapsed, &[KeyValue::new("endpoint", "/metrics-demo")]);
    info!(duration_ms = elapsed, "metrics_demo completed");
    HttpResponse::Ok()
        .json(serde_json::json!({ "message": "Metrics recorded", "duration_ms": elapsed }))
}

#[post("/echo")]
#[instrument(name = "echo_handler", skip(body, metrics))]
async fn echo(
    req: HttpRequest,
    body: web::Json<EchoPayload>,
    metrics: web::Data<AppMetrics>,
) -> impl Responder {
    let start = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let path = req.path().to_string();
    let method = req.method().to_string();
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", path.clone()),
            KeyValue::new("method", method),
        ],
    );
    info!(request_id = %request_id, payload = ?body, "Echo request received");
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    metrics
        .http_duration_ms
        .record(elapsed, &[KeyValue::new("endpoint", path)]);
    HttpResponse::Ok().json(EchoResponse {
        request_id,
        echoed: body.into_inner(),
    })
}

#[get("/trigger-error")]
#[instrument(name = "trigger_error")]
async fn trigger_error(metrics: web::Data<AppMetrics>) -> impl Responder {
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/trigger-error"),
            KeyValue::new("method", "GET"),
        ],
    );
    metrics.http_errors_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/trigger-error"),
            KeyValue::new("status", "500"),
        ],
    );
    metrics
        .stack_errors_total
        .add(1, &[KeyValue::new("error_type", "simulated")]);
    match do_work_that_fails().await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            // Log the full error chain for better stack trace visibility
            error!(
                error = %e,
                error_message = %e.to_string(),
                error_chain = ?e.chain().collect::<Vec<_>>(),
                endpoint = "/trigger-error",
                "Application error occurred with full stack trace"
            );
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string(),
                "error_chain": e.chain().map(|c| c.to_string()).collect::<Vec<_>>(),
                "request_id": Uuid::new_v4().to_string(),
            }))
        }
    }
}

#[get("/trigger-warning")]
#[instrument(name = "trigger_warning")]
async fn trigger_warning(metrics: web::Data<AppMetrics>) -> impl Responder {
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/trigger-warning"),
            KeyValue::new("method", "GET"),
        ],
    );
    warn!(
        endpoint = "/trigger-warning",
        threshold_exceeded = true,
        value = 9999,
        "High value detected"
    );
    HttpResponse::Ok().json(serde_json::json!({ "message": "Warning logged" }))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

#[instrument(name = "do_work_that_fails")]
async fn do_work_that_fails() -> anyhow::Result<()> {
    fetch_from_db().await.map_err(|e| {
        // Create a proper error chain with context
        anyhow::anyhow!("do_work_that_fails failed")
            .context(format!(
                "while processing user request at {}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ))
            .context(e)
    })
}

#[instrument(name = "fetch_from_db")]
async fn fetch_from_db() -> anyhow::Result<()> {
    // Provide more detailed error context
    Err(anyhow::anyhow!("database connection refused")
        .context("127.0.0.1:5432 unreachable")
        .context("attempting to fetch user data"))
}

// ─── Telemetry Init ───────────────────────────────────────────────────────────

fn resource(cfg: &OtelConfig) -> Resource {
    Resource::builder()
        .with_service_name("actix-openobserve")
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .with_attribute(KeyValue::new("deployment.environment", "development"))
        .with_attribute(KeyValue::new("openobserve.org", cfg.org_id.clone()))
        .build()
}

fn init_tracer(cfg: &OtelConfig) -> anyhow::Result<SdkTracerProvider> {
    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), cfg.auth.clone());
    headers.insert("organization".to_string(), cfg.org_id.clone());
    headers.insert("stream-name".to_string(), cfg.stream.clone());

    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(format!(
            "{}/api/{}/v1/traces",
            cfg.http_endpoint, cfg.org_id
        ))
        .with_headers(headers)
        .with_timeout(Duration::from_secs(5))
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(resource(cfg))
        .build();

    Ok(provider)
}

fn init_metrics(cfg: &OtelConfig) -> anyhow::Result<SdkMeterProvider> {
    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), cfg.auth.clone());
    headers.insert("organization".to_string(), cfg.org_id.clone());
    headers.insert("stream-name".to_string(), cfg.stream.clone());

    let exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(format!(
            "{}/api/{}/v1/metrics",
            cfg.http_endpoint, cfg.org_id
        ))
        .with_headers(headers)
        .with_timeout(Duration::from_secs(5))
        .build()?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(10))
        .build();

    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource(cfg))
        .build();

    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn init_logs(cfg: &OtelConfig) -> anyhow::Result<SdkLoggerProvider> {
    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), cfg.auth.clone());
    headers.insert("organization".to_string(), cfg.org_id.clone());
    headers.insert("stream-name".to_string(), cfg.stream.clone());

    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/api/{}/v1/logs", cfg.http_endpoint, cfg.org_id))
        .with_headers(headers)
        .with_timeout(Duration::from_secs(5))
        .build()?;

    let provider = SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource(cfg))
        .build();

    Ok(provider)
}

fn setup_tracing(cfg: &OtelConfig) -> anyhow::Result<(SdkTracerProvider, SdkLoggerProvider)> {
    let tracer_provider = init_tracer(cfg)?;
    let log_provider = init_logs(cfg)?;

    let tracer = tracer_provider.tracer("actix-openobserve");

    global::set_tracer_provider(tracer_provider.clone());

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().json())
        .with(OpenTelemetryLayer::new(tracer))
        .with(opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&log_provider))
        .init();

    Ok((tracer_provider, log_provider))
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if it exists
    if let Err(e) = dotenvy::dotenv() {
        info!("No .env file found or failed to load: {}", e);
    }

    let cfg = OtelConfig::from_env();

    let (tracer_provider, log_provider) = setup_tracing(&cfg)?;
    let metrics_provider = init_metrics(&cfg)?;
    let app_metrics = web::Data::new(AppMetrics::new());

    info!(
        http_endpoint = %cfg.http_endpoint,
        org = %cfg.org_id,
        stream = %cfg.stream,
        "OpenObserve telemetry initialized — HTTP/protobuf"
    );
    info!("Starting Actix-web server on http://0.0.0.0:8080");

    let metrics_clone = app_metrics.clone();

    // Create server with graceful shutdown handling
    let server = HttpServer::new(move || {
        App::new()
            .app_data(metrics_clone.clone())
            .service(health)
            .service(metrics_demo)
            .service(echo)
            .service(trigger_error)
            .service(trigger_warning)
    })
    .bind("0.0.0.0:8080")?
    .run();

    // Handle graceful shutdown
    let handle = server.handle();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("Shutting down server...");
        handle.stop(true).await;
    });

    // Run the server
    server.await?;

    // Properly shutdown telemetry providers to flush all data
    info!("Shutting down telemetry providers...");

    // Shutdown metrics provider
    if let Err(e) = metrics_provider.shutdown() {
        error!("Failed to shutdown metrics provider: {}", e);
    }

    // Shutdown log provider - flush remaining logs
    if let Err(e) = log_provider.shutdown() {
        error!("Failed to shutdown log provider: {}", e);
    }

    // Shutdown tracer provider - must happen before app exits to flush remaining spans
    if let Err(e) = tracer_provider.shutdown() {
        error!("Failed to shutdown tracer provider: {}", e);
    }

    info!("Telemetry shutdown complete");

    Ok(())
}
