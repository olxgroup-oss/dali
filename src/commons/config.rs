// (c) Copyright 2019-2024 OLX

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use serde::Serialize;
use std::env;
use std::fmt;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Configuration {
    pub app_port: u16,
    pub health_port: u16,
    pub log_level: Option<String>,
    pub vips_threads: Option<u16>,
    pub reqwest_timeout_millis: Option<u16>,
    pub reqwest_connection_timeout_millis: Option<u16>,
    pub reqwest_pool_max_idle_per_host: Option<u16>,
    pub reqwest_pool_idle_timeout_millis: Option<u16>,
    pub s3_region: Option<String>,
    pub s3_key: Option<String>,
    pub s3_secret: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_bucket: Option<String>,
    pub max_file_size: Option<u32>,
    pub otel_collector_endpoint: Option<String>,
    pub otel_application_name: Option<String>,
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}

impl Configuration {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let s = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(Environment::default())
            .build()?;
        s.try_deserialize()
    }
}
