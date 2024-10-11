use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use lazy_static::lazy_static;
use log::error;
use prometheus::{
    register_histogram_vec, register_int_counter, Encoder, HistogramVec, IntCounter, TextEncoder,
};
use prometheus_static_metric::make_static_metric;

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
    pub static ref FILES_EXCEEDING_MAX_ALLOWED_SIZE: IntCounter = register_int_counter!(
        "dali_files_exceeding_max_allowed_size",
        "Amount of files that were not processed due to exceeding the maximum allowed size"
    )
    .expect("Cannot register metric");
    pub static ref HTTP_DURATION: HttpRequestDuration =
        HttpRequestDuration::from(&HTTP_DURATION_VEC);
    pub static ref FETCH_DURATION: FetchRequestDuration =
        FetchRequestDuration::from(&FETCH_DURATION_VEC);
    pub static ref INPUT_SIZE: InputSize = InputSize::from(&INPUT_SIZE_VEC);
    pub static ref OUTPUT_SIZE: OutputSize = OutputSize::from(&OUTPUT_SIZE_VEC);
}

pub async fn handle_prometheus_scrapping() -> impl IntoResponse {
    let registry = prometheus::default_registry();
    let mut buffer = vec![];
    match TextEncoder::new().encode(&registry.gather(), &mut buffer) {
        Ok(()) => Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(buffer))
            .unwrap(),
        Err(e) => {
            error!(
                "failed to provide the metrics to the Prometheus server. error: {}",
                e
            );
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from("something went wrong on our side."))
                .unwrap()
        }
    }
}
