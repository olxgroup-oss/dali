// (c) Copyright 2019-2020 OLX
#[macro_use]
#[cfg(all(feature = "hyper_client", feature = "awc_client"))]
compile_error!("features `crate/hyper_client` and `crate/awc` are mutually exclusive");

#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(feature = "hyper_client")] {
        use hyper::{Client, client::HttpConnector};
        use hyper_timeout::TimeoutConnector;
        type DaliHttpClient = Client<TimeoutConnector<HttpConnector>>;
    } else {
        use awc::Client;
        type DaliHttpClient = Client;
    }
}

mod commons;
mod image_processor;

use commons::config::Configuration;
use commons::*;

use actix_service::Service;
use actix_web::{body, web::{self, Data}, App, HttpRequest, HttpResponse, HttpServer, error::ErrorInternalServerError};

use futures::future::join_all;
use image_processor::*;
use lazy_static::*;
use libvips::VipsApp;
use log::*;
use prometheus::*;
use prometheus_static_metric::*;
use std::{env, iter::once, time::SystemTime};

make_static_metric! {
    pub struct HttpRequestDuration: Histogram {
        "status" => {
            success,
            client_error,
            server_error,
        }
    }
    pub struct FetchRequestDuration: Histogram {
        "status" => {
            success,
        }
    }
    pub struct InputSize: Histogram {
        "format" => {
            jpeg,
            png,
            webp,
            heic,
        }
    }
    pub struct OutputSize: Histogram {
        "format" => {
            jpeg,
            png,
            webp,
            heic,
        }
    }
}

lazy_static! {
    pub static ref HTTP_DURATION_VEC: HistogramVec = register_histogram_vec!(
        "dali_http_requests_duration",
        "Duration of each HTTP request.",
        &["status"]
    )
    .expect("Cannot register metric");
    pub static ref FETCH_DURATION_VEC: HistogramVec = register_histogram_vec!(
        "dali_fetch_requests_duration",
        "Duration of the image fetch request(s).",
        &["status"]
    )
    .expect("Cannot register metric");
    pub static ref INPUT_SIZE_VEC: HistogramVec = register_histogram_vec!(
        "dali_input_size",
        "Number of bytes read from HTTP",
        &["format"]
    )
    .expect("Cannot register metric");
    pub static ref OUTPUT_SIZE_VEC: HistogramVec = register_histogram_vec!(
        "dali_output_size",
        "Number of bytes sent to clients",
        &["format"]
    )
    .expect("Cannot register metric");
    pub static ref HTTP_DURATION: HttpRequestDuration =
        HttpRequestDuration::from(&HTTP_DURATION_VEC);
    pub static ref FETCH_DURATION: FetchRequestDuration =
        FetchRequestDuration::from(&FETCH_DURATION_VEC);
    pub static ref INPUT_SIZE: InputSize = InputSize::from(&INPUT_SIZE_VEC);
    pub static ref OUTPUT_SIZE: OutputSize = OutputSize::from(&OUTPUT_SIZE_VEC);
}

async fn index(req: HttpRequest, query: ProcessImageRequest) -> actix_web::Result<HttpResponse> {
    // let query: ProcessImageRequest = match req.app_data::<Data<Config>>()
    //     .ok_or("Query configuration missing") {
    //     Ok(query) => query.get_ref().clone().deserialize_str(req.query_string()).map_err(actix_web::error::ErrorBadRequest)?,
    //     Err(e) => return Err(ErrorInternalServerError(e)),
    // };
    let config = match req.app_data::<Data<Configuration>>()
        .ok_or("Configuration missing") {
        Ok(config) => config.get_ref(),
        Err(e) => return Err(ErrorInternalServerError(e)),
    };
    let vips_data = req.app_data::<Data<libvips::VipsApp>>().unwrap().get_ref().clone();
    let http_client = req.app_data::<Data<DaliHttpClient>>().unwrap().get_ref().clone();
    let now = SystemTime::now();
    debug!("Request parameters: {:?}", query);
    let uri = req.uri();
    let format = query.format;
    let x_trace = String::from(
        req.headers()
            .get("x-trace")
            .map(|h| h.to_str().unwrap_or_else(|_| "EMPTY"))
            .unwrap_or_else(|| "EMPTY"),
    );

    let img_futures = query
        .watermarks
        .iter()
        .map(|wm| http::client::get_file(&http_client, &wm.image_address, &config));
    let main_img_fut = http::client::get_file(&http_client, &query.image_address, &config);
    let buffers = join_all(once(main_img_fut).chain(img_futures))
        .await
        .into_iter()
        .collect::<actix_web::Result<Vec<_>>>()?;
    if let Ok(elapsed) = now.elapsed() {
        let duration =
            (elapsed.as_secs() as f64) + f64::from(elapsed.subsec_nanos()) / 1_000_000_000_f64;
        FETCH_DURATION.success.observe(duration);
    }
    match web::block(move || {
        let mut input_size = 0;
        let result = process_image(&buffers[0], &buffers[1..], query, &mut input_size);
        match format {
            ImageFormat::Jpeg => INPUT_SIZE.jpeg.observe(input_size as f64),
            ImageFormat::Heic => INPUT_SIZE.heic.observe(input_size as f64),
            ImageFormat::Webp => INPUT_SIZE.webp.observe(input_size as f64),
            ImageFormat::Png => INPUT_SIZE.png.observe(input_size as f64),
        }
        result
    })
    .await?
    {
        Err(e) => {
            let error_str = format!("{}", e).replace("\"", "\\\"");
            error!(
                r#"{{"x-trace": "{}", "uri": "{}", "msg": "Error processing request: {}", "vips_error_buffer": "{}"}}"#,
                x_trace,
                uri,
                error_str,
                vips_data.error_buffer().unwrap_or("").replace("\n", ". ")
            );
            let error_response = match e {
                libvips::error::Error::InitializationError(_) => actix_web::error::ErrorBadRequest(e),
                _ => actix_web::error::ErrorInternalServerError(e),
            };
            Err(error_response)
        }
        Ok(res_body) => {
            match format {
                ImageFormat::Jpeg => OUTPUT_SIZE.jpeg.observe(res_body.len() as f64),
                ImageFormat::Heic => OUTPUT_SIZE.heic.observe(res_body.len() as f64),
                ImageFormat::Webp => OUTPUT_SIZE.webp.observe(res_body.len() as f64),
                ImageFormat::Png => OUTPUT_SIZE.png.observe(res_body.len() as f64),
            }
            Ok(HttpResponse::Ok()
                .content_type(format!("image/{}", format).as_str())
                .body(body::EitherBody::new(res_body)))
        }
    }
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn metrics() -> HttpResponse {
    let registry = prometheus::default_registry();
    let mut buffer = vec![];
    match TextEncoder::new().encode(&registry.gather(), &mut buffer) {
        Ok(()) => HttpResponse::Ok()
            .content_type("text/plain")
            .body(body::EitherBody::new(buffer)),
        Err(e) => HttpResponse::from_error(actix_web::error::ErrorInternalServerError(e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = Configuration::new().expect("Failed to load application configuration.");
    println!(r#"{{"configuration": {}}}"#, config);
    let config_data = web::Data::new(config);
    let name = "dali";
    env::set_var(
        "RUST_LOG",
        config_data
            .log_level
            .as_ref()
            .unwrap_or(&"info".to_string()),
    );
    env_logger::builder()
        .filter_module("actix_http::response", log::LevelFilter::Off)
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
    let cpus = num_cpus::get();
    let mut available_threads = if let Some(max_threads) = config_data.max_threads {
        max_threads
    } else {
        cpus as u16
    };
    if available_threads < 2 {
        available_threads = 2;
    }
    let vips_threads = if let Some(vips_threads) = config_data.vips_threads {
        vips_threads
    } else {
        available_threads / 2
    };
    let app_threads = if let Some(app_threads) = config_data.app_threads {
        app_threads as usize
    } else {
        available_threads as usize / 2
    };
    let metrics_threads = if let Some(metrics_threads) = config_data.metrics_threads {
        metrics_threads as usize
    } else {
        1
    };
    println!(
        r#"{{"num_cpus": {}, "application_threads": {}, "vips_threads": {}, "metrics_threads": {}, "message": "dali initialized" }}"#,
        cpus, app_threads, vips_threads, metrics_threads
    );
    let app = VipsApp::new(name, false).expect("Cannot initialize libvips");
    app.concurrency_set(vips_threads as i32);
    app.cache_set_max(0);
    app.cache_set_max_mem(0);

    //accept url encoded with brackets or their encoded equivalents
    let qs_config = serde_qs::Config::new(5, false);
    let qs_config_data = web::Data::new(qs_config);
    let vips_data = web::Data::new(app);
    let app_port = config_data.app_port;
    let health_port = config_data.health_port;

    let client_timeout = std::time::Duration::new(
        config_data.server_client_timeout.unwrap_or(5000), 0
    );
    let client_shutdown_timeout = std::time::Duration::new(
        config_data.client_shutdown_timeout.unwrap_or(5000), 0
    );
    let server_keep_alive = std::time::Duration::new(
        config_data.server_keep_alive.unwrap_or(7200) as u64, 0
    );

    let _server_metrics = HttpServer::new(move || {
        App::new()
            .service(web::resource("/metrics").route(web::get().to(metrics)))
            .service(web::resource("/health").route(web::get().to(health)))
    })
    .bind(format!("0.0.0.0:{}", health_port))?
    .workers(metrics_threads)
    .client_request_timeout( client_timeout)
    .client_disconnect_timeout(client_shutdown_timeout)
    .keep_alive(server_keep_alive)
    .run(); 

    let client_timeout = std::time::Duration::new(
        config_data.http_client_con_timeout.unwrap_or(5000), 0
    );

    #[cfg(feature = "hyper_client")]
    let http_client: DaliHttpClient = http::client::init_client(client_timeout.as_secs())
        .await
        .expect("Can't initilize http client");

    let server_main = HttpServer::new(move || {
        let mut app = App::new()
            .app_data(config_data.clone())
            .app_data(qs_config_data.clone())
            .app_data(vips_data.clone())
            .wrap_fn(|req, srv| {
                let now = SystemTime::now();
                let fut = srv.call(req);
                async move {
                    let res = fut.await?;
                    if let Ok(elapsed) = now.elapsed() {
                        let duration = (elapsed.as_secs() as f64) + f64::from(elapsed.subsec_nanos()) / 1_000_000_000_f64;
                        if res.status().is_client_error() {
                            HTTP_DURATION.client_error.observe(duration);
                        } else if res.status().is_server_error() {
                            HTTP_DURATION.server_error.observe(duration);
                        } else {
                            HTTP_DURATION.success.observe(duration);
                        }
                    }
                    Ok(res)
                }
            })
            .wrap(
                actix_web::middleware::Logger::new(
                    r#"{"remote-addr": "%a","status": %s,"content-length": "%b","request-duration": %D,"client-id": "%{X-ClientId}i","x-trace": "%{x-trace}i","referer": "%{Referer}i","user-agent": "%{User-Agent}i"}"#,
                ),
            )
            .service(web::resource("/").route(web::get().to(index)));

        // one global http client for all threads/workers
        #[cfg(feature = "hyper_client")]
        {
            app = app.app_data(Data::new(http_client.clone()));
        }

        // one http client per thread/worker
        #[cfg(feature = "awc_client")]
        {
            app = app.data_factory(move || {
                http::client::init_client(client_timeout.as_secs())
            });
        }

        app
    })
    .bind(format!("0.0.0.0:{}", app_port))?
    .workers(app_threads)
    .keep_alive(server_keep_alive)
    .client_request_timeout(client_timeout)
    .client_disconnect_timeout(client_shutdown_timeout)
    .run();

    server_main.await
}