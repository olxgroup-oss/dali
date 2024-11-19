// (c) Copyright 2019-2024 OLX
#![cfg(feature = "opentelemetry")]
use std::time::Duration;

use log::error;
use log::warn;
use opentelemetry::metrics::Meter;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_sdk::trace::{Config, RandomIdGenerator, Sampler};
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

    let trace_config = Config::default()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(16)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            otel_application_name,
        )]));

    let tracer_provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter.unwrap(), opentelemetry_sdk::runtime::Tokio)
        .with_config(trace_config)
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

    let metrics_reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter.unwrap(),
        opentelemetry_sdk::runtime::Tokio,
    )
    .build();

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(metrics_reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            otel_application_name,
        )]))
        .build();

    global::set_meter_provider(provider);
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

            system.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
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
