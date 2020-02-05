// (c) Copyright 2019-2020 OLX

use actix_web::web::Bytes;
use actix_web::Error;
use hyper::body::to_bytes;
use hyper::Uri;
use log::*;
use std::str::FromStr;

pub type HyperClient = hyper::Client<
    hyper_timeout::TimeoutConnector<
        hyper::client::HttpConnector<hyper::client::connect::dns::GaiResolver>,
    >,
    hyper::Body,
>;

pub async fn get_file(client: &HyperClient, url: &str) -> Result<Bytes, Error> {
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
        to_bytes(response.into_body()).await.map_err(move |e| {
            let error_str = format!("{}", e).replace("\"", "\\\"");
            error!("Error getting http file: {}", error_str);
            actix_web::error::InternalError::new(String::from("Error reading stream."), status)
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
