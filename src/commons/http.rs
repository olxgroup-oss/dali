// (c) Copyright 2019-2020 OLX

use actix_web::web::Bytes;
use actix_web::Error;
use actix_web::client::Client;
use log::*;

pub async fn get_file(client: &Client, url: &str) -> Result<Bytes, Error> {
    debug!("Fetching image from url {}", url);
    let mut response = client.get(url)
        .send()
        .await
        .map_err(move |e| {
            let error_str = format!("{}", e).replace("\"", "\\\"");
            error!("Error to send request: {}", error_str);
            actix_web::error::InternalError::new(String::from("Error to send request"), actix_http::http::StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    let status = response.status();
    if status.is_success() {
        response.body().await.map_err(move |e| {
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
