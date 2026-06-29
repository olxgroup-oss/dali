// (c) Copyright 2019-2026 OLX
use std::env;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use image_provider::{create_image_provider, ImageProvider};
use libvips::VipsApp;
use moka::future::Cache;

use commons::config::Configuration;
use routes::metric::HTTP_DURATION;

#[cfg(feature = "opentelemetry")]
use opentelemetry::trace::FutureExt;

mod commons;
mod image_processor;
mod image_provider;
mod routes;

#[tokio::main]
async fn main() {
    let config = Configuration::new().expect("Failed to load application configuration.");
    println!(r#"{{"configuration": {}}}"#, config);

    #[cfg(feature = "opentelemetry")]
    commons::open_telemetry::init_opentelemetry(&config).await;

    set_up_logging(&config);
    let (_, _) = tokio::join!(start_main_server(&config), start_management_server(&config));
}

fn set_up_logging(config: &Configuration) {
    env::set_var(
        "RUST_LOG",
        config.log_level.as_ref().unwrap_or(&"info".to_string()),
    );
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .format(|f, record| {
            use std::io::Write;
            let message = record.args().to_string();
            let as_json = match message.chars().next() {
                Some('{') => message,
                _ => format!(r#""{}""#, message),
            };
            writeln!(
                f,
                r#"{{"timestamp": {}, "level": "{}","target": "{}","message": {}}}"#,
                commons::timestamp_millis(),
                record.level(),
                record.target(),
                as_json,
            )
        })
        .init();
}

fn create_vips_app(config: &Configuration) -> Option<VipsApp> {
    let vips_threads = if let Some(vips_threads) = config.vips_threads {
        vips_threads
    } else {
        (num_cpus::get() / 2) as u16
    };

    let vips_app_name = "dali";
    let app = VipsApp::new(vips_app_name, false).expect("Cannot initialize libvips");
    app.concurrency_set(vips_threads as i32);
    app.cache_set_max(0);
    app.cache_set_max_mem(0);
    Some(app)
}

fn create_watermarks_cache(config: &Configuration) -> Cache<String, Arc<Vec<u8>>> {
    let cache_size = config.watermark_cache_size.unwrap_or(15);
    let ttl = Duration::from_secs(config.watermark_cache_ttl_seconds.unwrap_or(28800)); // 8 hours
    Cache::builder()
        .max_capacity(cache_size)
        .time_to_live(ttl)
        .build()
}

async fn start_management_server(config: &Configuration) {
    let app = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route("/metrics", get(routes::metric::handle_prometheus_scrapping));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.health_port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub struct AppState {
    vips_app: Arc<VipsApp>,
    image_provider: Arc<Box<dyn ImageProvider>>,
    config: Arc<Configuration>,
    watermark_cache: Cache<String, Arc<Vec<u8>>>,
}

async fn measure_request_handling_duration(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = SystemTime::now();

    #[cfg(feature = "opentelemetry")]
    let res = {
        let http_request_method = req.method().as_str().to_owned();
        let http_route = req
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .map(|matched_path| matched_path.as_str().to_owned())
            .unwrap_or_else(|| req.uri().path().to_owned());
        let url_scheme = req.uri().scheme_str().unwrap_or("http").to_owned();
        let url_path = req.uri().path().to_owned();
        let server_address = req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|host| host.to_str().ok())
            .map(str::to_owned);
        let network_protocol_version = match req.version() {
            axum::http::Version::HTTP_10 => "1.0",
            axum::http::Version::HTTP_11 => "1.1",
            axum::http::Version::HTTP_2 => "2",
            axum::http::Version::HTTP_3 => "3",
            _ => "",
        };

        let otel_cx = commons::open_telemetry::start_http_server_span(
            &http_request_method,
            &http_route,
            &url_scheme,
            &url_path,
            server_address.as_deref(),
            network_protocol_version,
        );
        let res = next.run(req).with_context(otel_cx.clone()).await;
        let duration = now
            .elapsed()
            .map(|elapsed| elapsed.as_secs_f64())
            .unwrap_or_default();
        commons::open_telemetry::finish_http_server_span(
            &otel_cx,
            &http_request_method,
            &http_route,
            &url_scheme,
            res.status().as_u16(),
            duration,
        );
        res
    };

    #[cfg(not(feature = "opentelemetry"))]
    let res = next.run(req).await;

    if let Ok(elapsed) = now.elapsed() {
        let duration =
            (elapsed.as_secs() as f64) + f64::from(elapsed.subsec_nanos()) / 1_000_000_000_f64;

        match res.status() {
            status if status.is_client_error() => HTTP_DURATION.client_error.observe(duration),
            status if status.is_server_error() => HTTP_DURATION.server_error.observe(duration),
            _ => HTTP_DURATION.success.observe(duration),
        }
    }
    Ok(res)
}

async fn start_main_server(config: &Configuration) {
    let app_state = AppState {
        vips_app: Arc::new(create_vips_app(&config).unwrap()),
        image_provider: Arc::new(create_image_provider(&config).await),
        config: Arc::new(config.clone()),
        watermark_cache: create_watermarks_cache(&config),
    };

    let app = Router::new()
        .route("/", get(routes::image::process_image))
        .with_state(app_state)
        .layer(middleware::from_fn(measure_request_handling_duration));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.app_port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
