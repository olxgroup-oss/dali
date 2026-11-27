// (c) Copyright 2019-2026 OLX
#![cfg(feature = "opentelemetry")]
use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

#[cfg(any(feature = "reqwest", test))]
use axum::http::HeaderValue;
use axum::http::{HeaderMap, HeaderName};
use log::warn;
use opentelemetry::metrics::{Histogram, MeterProvider};
#[cfg(feature = "reqwest")]
use opentelemetry::propagation::Injector;
use opentelemetry::propagation::{Extractor, TextMapCompositePropagator};
use opentelemetry::trace::{SpanKind, Status, TraceContextExt, Tracer, TracerProvider};
use opentelemetry::{global, Context, Key, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::trace::{RandomIdGenerator, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use sysinfo::{get_current_pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::task::JoinHandle;
use tokio::time;
use tonic::transport::ClientTlsConfig;

use super::config::Configuration;

const DEFAULT_OTEL_APPLICATION_NAME: &str = "dali";

#[derive(Debug)]
pub enum OpenTelemetryInitError {
    TraceExporter(String),
    MetricExporter(String),
    ProcessId(String),
}

impl fmt::Display for OpenTelemetryInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TraceExporter(error) => {
                write!(
                    f,
                    "failed to initialize the OpenTelemetry trace exporter: {error}"
                )
            }
            Self::MetricExporter(error) => {
                write!(
                    f,
                    "failed to initialize the OpenTelemetry metric exporter: {error}"
                )
            }
            Self::ProcessId(error) => {
                write!(f, "failed to initialize process memory metrics: {error}")
            }
        }
    }
}

impl std::error::Error for OpenTelemetryInitError {}

pub struct OpenTelemetryRuntime {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    memory_task: JoinHandle<()>,
}

impl OpenTelemetryRuntime {
    pub async fn shutdown(self) {
        self.memory_task.abort();
        let _ = self.memory_task.await;

        let shutdown_result = tokio::task::spawn_blocking(move || {
            if let Err(error) = self.meter_provider.shutdown() {
                warn!("failed to shut down the OpenTelemetry meter provider: {error}");
            }
            if let Err(error) = self.tracer_provider.shutdown() {
                warn!("failed to shut down the OpenTelemetry tracer provider: {error}");
            }
        })
        .await;

        if let Err(error) = shutdown_result {
            warn!("OpenTelemetry shutdown task failed: {error}");
        }
    }
}

pub async fn init_opentelemetry(
    config: &Configuration,
) -> Result<Option<OpenTelemetryRuntime>, OpenTelemetryInitError> {
    let Some(endpoint) = config.otel_collector_endpoint.clone() else {
        warn!("the endpoint of the OpenTelemetry Collector is missing from the configuration parameters. OpenTelemetry won't be enabled.");
        return Ok(None);
    };

    let otel_application_name = config
        .otel_application_name
        .clone()
        .unwrap_or(DEFAULT_OTEL_APPLICATION_NAME.to_owned());
    let resource = build_resource(otel_application_name);

    install_propagators();
    let tracer_provider = init_global_tracer_provider(endpoint.clone(), resource.clone())?;
    let meter_provider = init_global_meter_provider(endpoint, resource)?;
    let memory_task = schedule_process_memory_metrics(&meter_provider)?;

    Ok(Some(OpenTelemetryRuntime {
        tracer_provider,
        meter_provider,
        memory_task,
    }))
}

fn init_global_tracer_provider(
    otel_collector_endpoint: String,
    resource: Resource,
) -> Result<SdkTracerProvider, OpenTelemetryInitError> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(otel_collector_endpoint)
        .with_tls_config(ClientTlsConfig::new().with_native_roots())
        .with_timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| OpenTelemetryInitError::TraceExporter(error.to_string()))?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(16)
        .with_resource(resource)
        .build();

    global::set_tracer_provider(tracer_provider.clone());
    Ok(tracer_provider)
}

fn init_global_meter_provider(
    otel_collector_endpoint: String,
    resource: Resource,
) -> Result<SdkMeterProvider, OpenTelemetryInitError> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_temporality(opentelemetry_sdk::metrics::Temporality::Delta)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .with_endpoint(otel_collector_endpoint)
        .with_tls_config(ClientTlsConfig::new().with_native_roots())
        .with_timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| OpenTelemetryInitError::MetricExporter(error.to_string()))?;

    let metrics_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter).build();

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(metrics_reader)
        .with_resource(resource)
        .build();

    global::set_meter_provider(provider.clone());

    init_http_server_request_duration_metric();
    Ok(provider)
}

static HTTP_SERVER_REQUEST_DURATION: OnceLock<Histogram<f64>> = OnceLock::new();

const OTEL_INSTRUMENTATION_SCOPE_NAME: &str = "dali";

fn install_propagators() {
    global::set_text_map_propagator(TextMapCompositePropagator::new(vec![
        Box::new(BaggagePropagator::new()),
        Box::new(TraceContextPropagator::new()),
    ]));
}

fn build_resource(otel_application_name: String) -> Resource {
    let hostname = System::host_name();

    let detected = Resource::builder().build();
    let has_host_name = detected.get(&Key::from_static_str("host.name")).is_some();
    let has_instance_id = detected
        .get(&Key::from_static_str("service.instance.id"))
        .is_some();
    let has_service_version = detected
        .get(&Key::from_static_str("service.version"))
        .is_some();

    let mut builder = Resource::builder().with_service_name(otel_application_name);

    if !has_host_name {
        if let Some(hostname) = hostname.clone() {
            builder = builder.with_attribute(KeyValue::new("host.name", hostname));
        }
    }

    if !has_instance_id {
        let instance_id = match hostname {
            Some(hostname) => format!("{hostname}-{}", std::process::id()),
            None => std::process::id().to_string(),
        };
        builder = builder.with_attribute(KeyValue::new("service.instance.id", instance_id));
    }
    if !has_service_version {
        builder =
            builder.with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")));
    }

    builder.build()
}

fn init_http_server_request_duration_metric() {
    let meter = global::meter_provider().meter(OTEL_INSTRUMENTATION_SCOPE_NAME);
    let histogram = meter
        .f64_histogram("http.server.request.duration")
        .with_unit("s")
        .with_description("Duration of HTTP server requests.")
        .with_boundaries(vec![
            0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.125, 0.15, 0.175, 0.2, 0.25, 0.3, 0.4, 0.5,
            0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
        ])
        .build();
    if HTTP_SERVER_REQUEST_DURATION.set(histogram).is_err() {
        warn!("the http.server.request.duration metric has already been initialized");
    }
}

struct HeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

#[cfg(feature = "reqwest")]
struct HeaderInjector<'a>(&'a mut HeaderMap);

#[cfg(feature = "reqwest")]
impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        let Ok(name) = HeaderName::from_bytes(key.as_bytes()) else {
            return;
        };
        let Ok(value) = HeaderValue::from_str(&value) else {
            return;
        };
        self.0.insert(name, value);
    }
}

pub fn extract_http_context(headers: &HeaderMap) -> Context {
    global::get_text_map_propagator(|propagator| {
        propagator.extract_with_context(&Context::new(), &HeaderExtractor(headers))
    })
}

#[cfg(feature = "reqwest")]
pub fn inject_http_context(context: &Context, headers: &mut HeaderMap) {
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, &mut HeaderInjector(headers));
    });
}

pub fn start_http_server_span(
    parent_context: &Context,
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
        .start_with_context(&tracer, parent_context);

    parent_context.with_span(span)
}

pub fn start_client_span(name: String, attributes: Vec<KeyValue>) -> Context {
    let tracer = global::tracer_provider().tracer(OTEL_INSTRUMENTATION_SCOPE_NAME);
    let parent_context = Context::current();
    let span = tracer
        .span_builder(name)
        .with_kind(SpanKind::Client)
        .with_attributes(attributes)
        .start_with_context(&tracer, &parent_context);

    parent_context.with_span(span)
}

pub fn finish_client_span(
    context: &Context,
    error_type: Option<String>,
    error_message: Option<String>,
) {
    let span = context.span();
    if let Some(error_type) = error_type {
        span.set_attribute(KeyValue::new("error.type", error_type));
        span.set_status(Status::error(
            error_message.unwrap_or_else(|| "client operation failed".to_owned()),
        ));
    }
    span.end();
}

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

fn schedule_process_memory_metrics(
    meter_provider: &SdkMeterProvider,
) -> Result<JoinHandle<()>, OpenTelemetryInitError> {
    let pid =
        get_current_pid().map_err(|error| OpenTelemetryInitError::ProcessId(error.to_owned()))?;
    let meter = meter_provider.meter(OTEL_INSTRUMENTATION_SCOPE_NAME);
    let memory_gauge = meter
        .u64_gauge("process.memory.usage")
        .with_unit("By")
        .with_description("The amount of physical memory in use by the Dali process.")
        .build();

    Ok(tokio::spawn(async move {
        let mut system = System::new();
        let mut interval = time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            if let Some(memory) = process_memory_bytes(&mut system, pid) {
                memory_gauge.record(memory, &[]);
            }
        }
    }))
}

fn process_memory_bytes(system: &mut System, pid: sysinfo::Pid) -> Option<u64> {
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().with_memory(),
    );
    system.process(pid).map(|process| process.memory())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState};
    use opentelemetry::Value;

    const TRACE_ID: &str = "4bf92f3577b34da6a3ce929d0e0e4736";
    const SPAN_ID: &str = "00f067aa0ba902b7";
    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn resource_contains_service_identity() {
        let resource = build_resource("dali-test".to_owned());

        assert_eq!(
            resource.get(&Key::from_static_str("service.name")),
            Some(Value::from("dali-test"))
        );
        assert!(resource
            .get(&Key::from_static_str("service.version"))
            .is_some());
    }

    #[test]
    fn extracts_w3c_parent_context() {
        install_propagators();
        let mut headers = HeaderMap::new();
        headers.insert("traceparent", HeaderValue::from_static(TRACEPARENT));

        let context = extract_http_context(&headers);
        let span_context = context.span().span_context().clone();

        assert_eq!(
            span_context.trace_id(),
            TraceId::from_hex(TRACE_ID).unwrap()
        );
        assert_eq!(span_context.span_id(), SpanId::from_hex(SPAN_ID).unwrap());
        assert!(span_context.is_remote());
    }

    #[test]
    #[cfg(feature = "reqwest")]
    fn injects_w3c_child_context() {
        install_propagators();
        let parent_span_context = SpanContext::new(
            TraceId::from_hex(TRACE_ID).unwrap(),
            SpanId::from_hex(SPAN_ID).unwrap(),
            TraceFlags::SAMPLED,
            true,
            TraceState::default(),
        );
        let parent_context = Context::new().with_remote_span_context(parent_span_context);
        let provider = SdkTracerProvider::builder().build();
        let tracer = provider.tracer("test");
        let child_span = tracer.start_with_context("child", &parent_context);
        let child_context = parent_context.with_span(child_span);
        let mut headers = HeaderMap::new();

        inject_http_context(&child_context, &mut headers);

        let traceparent = headers.get("traceparent").unwrap().to_str().unwrap();
        let parts: Vec<_> = traceparent.split('-').collect();
        assert_eq!(parts[1], TRACE_ID);
        assert_ne!(parts[2], SPAN_ID);
        assert_eq!(parts[3], "01");
    }

    #[test]
    fn reads_current_process_memory() {
        let pid = get_current_pid().expect("current pid should be available");
        let mut system = System::new();

        assert!(process_memory_bytes(&mut system, pid).is_some());
    }
}
