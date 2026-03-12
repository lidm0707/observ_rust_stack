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
use sentry::{ClientOptions, types::Dsn};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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

#[derive(Clone)]
struct SentryConfig {
    dsn: String,
    environment: String,
    release: String,
    sample_rate: f32,
}

impl SentryConfig {
    fn from_env() -> Self {
        let dsn = std::env::var("SENTRY_DSN").expect("not found sentry dsn");
        let environment =
            std::env::var("SENTRY_ENVIRONMENT").expect("not found SENTRY_ENVIRONMENT");
        let release = std::env::var("SENTRY_RELEASE").expect("not found SENTRY_RELEASE");
        let sample_rate = std::env::var("SENTRY_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        SentryConfig {
            dsn,
            environment,
            release,
            sample_rate,
        }
    }
}

impl OtelConfig {
    fn from_env() -> Self {
        let http_endpoint =
            std::env::var("OPENOBSERVE_HTTP_ENDPOINT").expect("OPENOBSERVE_HTTP_ENDPOINT");

        let org_id = std::env::var("OPENOBSERVE_ORG").expect("OPENOBSERVE_ORG");
        let stream = std::env::var("OPENOBSERVE_STREAM").expect("OPENOBSERVE_STREAM");
        let user = std::env::var("OPENOBSERVE_USER").expect("OPENOBSERVE_USER");
        let pass = std::env::var("OPENOBSERVE_PASS").expect("OPENOBSERVE_PASS");
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
#[instrument(name = "trigger_error", skip(metrics))]
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

            // Capture error in Sentry/Glitchtip if initialized
            let event_id = sentry::capture_message(&e.to_string(), sentry::Level::Error);
            info!(event_id = %event_id, "Error captured in Sentry/Glitchtip");
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

#[get("/trigger-panic")]
#[instrument(name = "trigger_panic", skip(metrics))]
async fn trigger_panic(metrics: web::Data<AppMetrics>) -> impl Responder {
    metrics.http_requests_total.add(
        1,
        &[
            KeyValue::new("endpoint", "/trigger-panic"),
            KeyValue::new("method", "GET"),
        ],
    );

    let request_id = Uuid::new_v4().to_string();
    let request_id_for_panic = request_id.clone();

    error!(
        endpoint = "/trigger-panic",
        %request_id,
        "About to trigger a panic - this should be captured by Glitchtip"
    );

    // Spawn a thread to trigger the panic so Actix-web's panic handler doesn't interfere
    // Sentry's PanicIntegration will capture this panic
    std::thread::spawn(move || {
        panic!("Oh no, an error! Request ID: {}", request_id_for_panic);
    });

    // Give the panic thread time to execute and send to Sentry
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Return a response to the client
    HttpResponse::InternalServerError().json(serde_json::json!({
        "error": "Panic triggered in background thread - check Glitchtip!",
        "request_id": request_id,
        "note": "Panic occurred in separate thread, Sentry PanicIntegration should have captured it"
    }))
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

fn init_sentry(cfg: &SentryConfig) -> Option<sentry::ClientInitGuard> {
    // Simple Sentry initialization following GlitchTip documentation
    // This is non-blocking and will not prevent app startup even if DSN is invalid

    // Parse DSN first - if it fails, return None and continue app startup
    let dsn = match Dsn::from_str(&cfg.dsn) {
        Ok(dsn) => dsn,
        Err(e) => {
            warn!(
                "Invalid Sentry DSN '{}': {}. Sentry error tracking disabled.",
                cfg.dsn, e
            );
            return None;
        }
    };

    let guard = sentry::init((
        dsn,
        ClientOptions {
            release: Some(cfg.release.clone().into()),
            environment: Some(cfg.environment.clone().into()),
            sample_rate: cfg.sample_rate,
            attach_stacktrace: true,
            send_default_pii: false,
            traces_sample_rate: 0.1,
            ..Default::default()
        }
        .add_integration(sentry::integrations::panic::PanicIntegration::default()),
    ));

    sentry::configure_scope(|scope| {
        scope.set_tag("service.name", "actix-openobserve");
        scope.set_tag("service.version", env!("CARGO_PKG_VERSION"));
        scope.set_tag("deployment.environment", &cfg.environment);
    });

    Some(guard)
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if it exists
    println!("BOOTING ACTIX APP");
    if let Err(e) = dotenvy::dotenv() {
        info!("No .env file found or failed to load: {}", e);
    }

    let cfg = OtelConfig::from_env();
    let sentry_cfg = SentryConfig::from_env();

    let (tracer_provider, log_provider) = setup_tracing(&cfg)?;
    let metrics_provider = init_metrics(&cfg)?;
    let app_metrics = web::Data::new(AppMetrics::new());

    // Initialize Sentry/Glitchtip (optional - app continues even if this fails)
    let _sentry_guard = init_sentry(&sentry_cfg);

    info!(
        http_endpoint = %cfg.http_endpoint,
        org = %cfg.org_id,
        stream = %cfg.stream,
        "OpenObserve telemetry initialized — HTTP/protobuf"
    );

    if _sentry_guard.is_some() {
        info!(
            sentry_dsn = %sentry_cfg.dsn,
            environment = %sentry_cfg.environment,
            release = %sentry_cfg.release,
            "Sentry/Glitchtip error tracking initialized"
        );
    } else {
        warn!(
            sentry_dsn = %sentry_cfg.dsn,
            "Sentry/Glitchtip error tracking NOT initialized - errors will only be logged to console"
        );
    }
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
            .service(trigger_panic)
    })
    .bind("0.0.0.0:8080")?
    .run();

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

    // Flush Sentry events before shutdown to ensure all captured errors are sent to GlitchTip
    if let Some(client) = sentry::Hub::current().client() {
        client.close(Some(std::time::Duration::from_secs(2)));
    }

    // Sentry shutdown happens automatically when _sentry_guard goes out of scope
    info!("Sentry/Glitchtip shutdown complete");
    info!("Telemetry shutdown complete");

    Ok(())
}
