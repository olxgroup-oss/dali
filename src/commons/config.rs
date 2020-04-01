// (c) Copyright 2019-2020 OLX

use config::{Config, ConfigError, Environment, File};
use serde_derive::*;
use std::env;
use std::fmt;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub app_port: u16,
    pub health_port: u16,
    pub log_level: Option<String>,
    pub server_client_timeout: Option<u64>,
    pub client_shutdown_timeout: Option<u64>,
    pub server_keep_alive: Option<usize>,
    pub http_client_con_timeout: Option<u64>,
    pub http_client_read_timeout: Option<u64>,
    pub http_client_write_timeout: Option<u64>,
    // https://docs.rs/awc/2.0.0-alpha.1/awc/struct.MessageBody.html#method.limit
    pub http_client_max_size_of_payload: Option<u64>,
    pub max_threads: Option<u16>,
    pub vips_threads: Option<u16>,
    pub app_threads: Option<u16>,
    pub metrics_threads: Option<u16>,
}

impl fmt::Display for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}

impl Configuration {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::with_name("config/default").required(false))?;

        let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;

        s.merge(Environment::new())?;

        s.try_into()
    }
}
