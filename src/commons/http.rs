// (c) Copyright 2019-2023 OLX

use crate::commons::config::Configuration;
use actix_web::web::Bytes;
use actix_web::Error;
use log::*;
use std::time::Duration;

// This is the incumbent `hyper_client` default feature.
// When choosing feature = `awc_cleint`; `hyper_client` is also present, as it is a default
// Therefore, we use the not `awc_client` feature to enable the default behaviour
#[cfg(not(feature = "awc_client"))]
pub mod client {
    use super::*;
    use hyper::body::{to_bytes, Buf};
    use hyper::Uri;
    use std::str::FromStr;

    pub type HttpClient = hyper::Client<
        hyper_timeout::TimeoutConnector<
            hyper::client::HttpConnector<hyper::client::connect::dns::GaiResolver>,
        >,
        hyper::Body,
    >;

    pub async fn init_client(http_client_timeout: Duration) -> Result<HttpClient, Error> {
        let hyper_builder = hyper::Client::builder();
        let http_connector = hyper::client::HttpConnector::new();
        let mut http_timeout_connector = hyper_timeout::TimeoutConnector::new(http_connector);
        http_timeout_connector.set_connect_timeout(Some(http_client_timeout));
        http_timeout_connector.set_write_timeout(Some(http_client_timeout));
        http_timeout_connector.set_read_timeout(Some(http_client_timeout));
        let hyper_http_client = hyper_builder.build::<_, hyper::Body>(http_timeout_connector);
        Ok(hyper_http_client)
    }

    pub async fn get_file(
        client: &HttpClient,
        url: &str,
        _config: &Configuration,
    ) -> Result<Bytes, Error> {
        debug!("Fetching image from url {}", url);
        let uri = Uri::from_str(url).map_err(|e| {
            let error_str = format!("{}", e).replace("\"", "\\\"");
            error!("Error parsing URI: {}", error_str);
            actix_web::error::ErrorInternalServerError(e)
        })?;
        let response = client.get(uri).await.map_err(move |e| {
            if let Some(cause) = e.into_cause() {
                if cause.is::<std::io::Error>() {
                    if let Some(timeout) = cause.downcast_ref::<std::io::Error>() {
                        if timeout.kind() == std::io::ErrorKind::TimedOut {
                            error!("Request for image {} timed out", url);
                            return actix_web::error::ErrorServiceUnavailable(cause);
                        }
                    }
                }
                let error_str = format!("{}", cause).replace("\"", "\\\"");
                error!("Error getting http file: {}: {}", url, error_str);
                actix_web::error::ErrorInternalServerError(cause)
            } else {
                actix_web::error::ErrorInternalServerError("Unknown error...")
            }
        })?;
        let status = response.status();
        if response.status().is_success() {
            match to_bytes(response.into_body()).await {
                Ok(bytes) => Ok(Bytes::from(bytes.chunk().to_owned())),
                Err(e) => {
                    let error_str = format!("{}", e).replace("\"", "\\\"");
                    error!("Error getting http file: {}", error_str);
                    Err(actix_web::error::InternalError::new(
                        String::from("Error reading stream."),
                        status,
                    )
                    .into())
                }
            }
        } else {
            error!("Error getting http file {}: {}", status, url);
            Err(actix_web::error::InternalError::new(
                format!("Error fetching file: {}", status),
                status,
            )
            .into())
        }
    }
}

#[cfg(feature = "awc_client")]
pub mod client {
    use super::*;
    use std::thread;

    pub type HttpClient = awc::Client;

    pub async fn init_client(http_client_timeout: u64) -> Result<HttpClient, Error> {
        info!("Configure http client for {:?}", thread::current().id());
        let client = HttpClient::builder()
            .timeout(Duration::from_millis(http_client_timeout))
            .finish();
        Ok(client)
    }

    pub async fn get_file(
        client: &HttpClient,
        url: &str,
        config: &Configuration,
    ) -> Result<Bytes, Error> {
        debug!("Fetching image from url {}", url);
        let mut response = client.get(url).send().await.map_err(move |e| {
            let error_str = format!("{}", e).replace("\"", "\\\"");
            error!("Error to send request: {}", error_str);
            actix_web::error::InternalError::new(
                String::from("Error to send request"),
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let status = response.status();
        if status.is_success() {
            response
                .body()
                // https://docs.rs/awc/2.0.0-alpha.1/awc/struct.MessageBody.html#method.limit
                .limit(config.http_client_max_size_of_payload.unwrap_or(1024 * 1024) as usize)
                .await
                .map_err(move |e| {
                    let error_str = format!("{}", e).replace("\"", "\\\"");
                    error!("Error getting http file: {}", error_str);
                    actix_web::error::InternalError::new(
                        String::from("Error reading stream."),
                        status,
                    )
                    .into()
                })
        } else {
            error!("Error getting http file {}: {}", status, url);
            Err(actix_web::error::InternalError::new(
                format!("Error fetching file: {}", status),
                status,
            )
            .into())
        }
    }
}
