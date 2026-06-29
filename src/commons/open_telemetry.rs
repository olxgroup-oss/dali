// (c) Copyright 2019-2026 OLX
#![cfg(feature = "opentelemetry")]
use std::sync::OnceLock;
use std::time::Duration;

use log::error;
use log::warn;
use opentelemetry::metrics::{Histogram, Meter};
use opentelemetry::trace::{SpanKind, Status, TraceContextExt, Tracer, TracerProvider};
use opentelemetry::{global, Context, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use sysinfo::{MemoryRefreshKind, System};
use tokio::{task, time};
use tonic::transport::ClientTlsConfig;

use super::config::Configuration;

pub const DEFAULT_OTEL_APPLICAITON_NAME: &str = "dali";

pub async fn init_opentelemetry(config: &Configuration) {
    let otel_collector_endpoint = config.otel_collector_endpoint.clone();
    if otel_collector_endpoint.is_none() {
        warn!("the endpoint of the OpenTelemetry Collector is missing from the configuration parameters. OpenTelemetry won't be enabled.");
        return;
    }
    let otel_application_name = config
        .otel_application_name
        .clone()
        .unwrap_or(DEFAULT_OTEL_APPLICAITON_NAME.to_owned());
    let endpoint = otel_collector_endpoint.unwrap();
    init_global_tracer_provider(endpoint.clone(), otel_application_name.clone());
    init_global_meter_provider(endpoint.clone(), otel_application_name.clone()).await;
    schedule_memory_metrics().await;
}

fn init_global_tracer_provider(otel_collector_endpoint: String, otel_application_name: String) {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(otel_collector_endpoint)
        .with_tls_config(ClientTlsConfig::new().with_native_roots())
        .with_timeout(Duration::from_secs(3))
        .build();

    if exporter.is_err() {
        error!(
            "failed to initiate the span exporter for opentelemetry. received error {}",
            exporter.unwrap_err()
        );
        return;
    }

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter.unwrap())
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(16)
        .with_resource(build_resource(otel_application_name))
        .build();

    global::set_tracer_provider(tracer_provider);
}

async fn init_global_meter_provider(
    otel_collector_endpoint: String,
    otel_application_name: String,
) {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(otel_collector_endpoint)
        .with_tls_config(ClientTlsConfig::new().with_native_roots())
        .with_timeout(Duration::from_secs(3))
        .build();

    if exporter.is_err() {
        error!(
            "failed to initiate the metric exporter for opentelemetry. received error {}",
            exporter.unwrap_err()
        );
        return;
    }

    let metrics_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter.unwrap())
        .build();

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(metrics_reader)
        .with_resource(build_resource(otel_application_name))
        .build();

    global::set_meter_provider(provider);

    init_http_server_request_duration_metric();
}

static HTTP_SERVER_REQUEST_DURATION: OnceLock<Histogram<f64>> = OnceLock::new();

const OTEL_INSTRUMENTATION_SCOPE_NAME: &str = "dali";

fn build_resource(otel_application_name: String) -> Resource {
    let mut builder = Resource::builder().with_service_name(otel_application_name);
    // `host.name` and `service.instance.id` are part of the resource attributes
    // that New Relic copies onto the derived APM metrics and uses to identify the
    // service instance.
    if let Some(hostname) = System::host_name() {
        builder = builder
            .with_attribute(KeyValue::new("host.name", hostname.clone()))
            .with_attribute(KeyValue::new("service.instance.id", hostname));
    }
    builder.build()
}

fn init_http_server_request_duration_metric() {
    let meter = global::meter_provider().meter(OTEL_INSTRUMENTATION_SCOPE_NAME);
    let histogram = meter
        .f64_histogram("http.server.request.duration")
        .with_unit("s")
        .with_description("Duration of HTTP server requests.")
        .build();
    if HTTP_SERVER_REQUEST_DURATION.set(histogram).is_err() {
        warn!("the http.server.request.duration metric has already been initialized");
    }
}

// Starts an OpenTelemetry server span following the HTTP semantic conventions.
// New Relic derives APM transactions, distributed traces and the errors inbox
// from these conventions, so the entry span must be of kind `Server` and expose
// the `http.*`/`url.*` attributes. The returned `Context` carries the span so it
// can be attached to the request handling future, making any span created while
// handling the request a child of this entry span.
pub fn start_http_server_span(
    http_request_method: &str,
    http_route: &str,
    url_scheme: &str,
    url_path: &str,
    server_address: Option<&str>,
    network_protocol_version: &str,
) -> Context {
    let tracer = global::tracer_provider().tracer(OTEL_INSTRUMENTATION_SCOPE_NAME);
    let mut attributes = vec![
        KeyValue::new("http.request.method", http_request_method.to_string()),
        KeyValue::new("http.route", http_route.to_string()),
        KeyValue::new("url.path", url_path.to_string()),
        KeyValue::new("url.scheme", url_scheme.to_string()),
    ];
    if let Some(server_address) = server_address {
        attributes.push(KeyValue::new("server.address", server_address.to_string()));
    }
    if !network_protocol_version.is_empty() {
        attributes.push(KeyValue::new(
            "network.protocol.version",
            network_protocol_version.to_string(),
        ));
    }

    let span = tracer
        .span_builder(format!("{http_request_method} {http_route}"))
        .with_kind(SpanKind::Server)
        .with_attributes(attributes)
        .start(&tracer);

    Context::current_with_span(span)
}

// Completes the server span started by `start_http_server_span` and records the
// `http.server.request.duration` metric. New Relic relies on this metric (and the
// `error.type` attribute) to drive throughput, response time and error rate, and
// on the span status to populate the errors inbox.
pub fn finish_http_server_span(
    otel_cx: &Context,
    http_request_method: &str,
    http_route: &str,
    url_scheme: &str,
    http_response_status_code: u16,
    duration_seconds: f64,
) {
    let span = otel_cx.span();
    span.set_attribute(KeyValue::new(
        "http.response.status_code",
        i64::from(http_response_status_code),
    ));
    // Per the OpenTelemetry HTTP semantic conventions only 5xx responses mark a
    // server span as errored; 4xx are client faults and must not inflate the
    // service error rate.
    if http_response_status_code >= 500 {
        span.set_attribute(KeyValue::new(
            "error.type",
            http_response_status_code.to_string(),
        ));
        span.set_status(Status::error(format!(
            "request failed with status code {http_response_status_code}"
        )));
    }
    span.end();

    record_http_server_request_duration(
        http_request_method,
        http_route,
        url_scheme,
        http_response_status_code,
        duration_seconds,
    );
}

fn record_http_server_request_duration(
    http_request_method: &str,
    http_route: &str,
    url_scheme: &str,
    http_response_status_code: u16,
    duration_seconds: f64,
) {
    let Some(histogram) = HTTP_SERVER_REQUEST_DURATION.get() else {
        return;
    };

    let mut attributes = vec![
        KeyValue::new("http.request.method", http_request_method.to_string()),
        KeyValue::new("http.route", http_route.to_string()),
        KeyValue::new("url.scheme", url_scheme.to_string()),
        KeyValue::new(
            "http.response.status_code",
            i64::from(http_response_status_code),
        ),
    ];
    if http_response_status_code >= 500 {
        attributes.push(KeyValue::new(
            "error.type",
            http_response_status_code.to_string(),
        ));
    }

    histogram.record(duration_seconds, &attributes);
}

async fn schedule_memory_metrics() {
    let _memory_task = task::spawn(async {
        let meter: Meter = global::meter_provider().meter("Dali - Memory Meter");

        let used_memory_gauge = meter
            .u64_gauge("dali_used_memory_mb")
            .with_description("Used memory in megabytes (MB)")
            .build();

        let free_memory_gauge = meter
            .u64_gauge("dali_free_memory_mb")
            .with_description("Free memory in megabytes (MB)")
            .build();

        let mut system = System::new_all();
        let mut interval = time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
            free_memory_gauge.record(
                system.free_memory() / 1_000_000,
                &[KeyValue::new("type", "free")],
            );
            used_memory_gauge.record(
                system.used_memory() / 1_000_000,
                &[KeyValue::new("type", "used")],
            );
        }
    });
}
