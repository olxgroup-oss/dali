// (c) Copyright 2019-2025 OLX
use std::env;
use std::sync::Arc;
use std::time::SystemTime;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use image_provider::{create_image_provider, ImageProvider};
use libvips::VipsApp;
use log::debug;

use commons::config::Configuration;
use routes::metric::HTTP_DURATION;

mod commons;
mod image_processor;
mod image_provider;
mod routes;

#[tokio::main]
async fn main() {
    let config = Configuration::new().expect("Failed to load application configuration.");
    debug!(r#"{{"configuration": {}}}"#, config);

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
}

async fn measure_request_handling_duration(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = SystemTime::now();
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
    };

    let app = Router::new()
        .route("/", get(routes::image::process_image))
        .with_state(app_state)
        .layer(middleware::from_fn(measure_request_handling_duration));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.app_port))
        .await
        .unwrap();
    println!("Starting Server at port: {}", config.app_port);

    axum::serve(listener, app).await.unwrap();
}
