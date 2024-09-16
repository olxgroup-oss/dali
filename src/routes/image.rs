use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequest, Request, State},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use core::str;
use futures::future::join_all;
use log::{error, warn};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::time::SystemTime;
use thiserror::Error;

use crate::{
    commons::{ImageFormat, ProcessImageRequest},
    image_processor, AppState,
};

use super::metric::{FETCH_DURATION, INPUT_SIZE, OUTPUT_SIZE};

// The following response headers are determined by Dali as it formats the image dowloaded from the provided source.
// Thus the length and type of the resulted image might be different compared to what the storage engine has returned.
// To match different variations regarding the case (lower/upper) they're specified in lowercase here and we convert
// to lower the other ones that we compare with.
const HEADERS_DETERMINED_BY_DALI: [&str; 2] = ["content-type", "content-length"];

pub struct ProcessImageRequestExtractor<T>(pub T);

#[async_trait]
impl<B, T> FromRequest<B> for ProcessImageRequestExtractor<T>
where
    B: Send,
    T: DeserializeOwned + Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, _state: &B) -> Result<Self, Self::Rejection> {
        let query = req.uri().query();
        if let Some(query) = query {
            let extracted_params = serde_qs::from_str(query);
            if extracted_params.is_ok() {
                Ok(Self(extracted_params.unwrap()))
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    format!("the provided parameters within the query string aren't valid"),
                ))
            }
        } else {
            Err((
                StatusCode::BAD_REQUEST,
                format!("the provided parameters within the query string aren't valid"),
            ))
        }
    }
}

#[derive(Error, Debug)]
pub enum ImageProcessingError {
    #[error("the provded resource uri is not valid: `{0}`")]
    InvalidResourceUriProvided(String),
    #[error("the download of the image timed out")]
    ImageDownloadTimedOut,
    #[error("received error response `{0}` while attempting to download the image `{1}`")]
    ClientReturnedErrorStatusCode(u16, String),
    #[error("the download of the image has failed")]
    ImageDownloadFailed,
    #[error("failed to join the thread that was doing the processing")]
    ProcessingWorkerJoinError,
    #[error("the image processing with libvips has failed")]
    LibvipsProcessingFailed(libvips::error::Error),
}

impl IntoResponse for ImageProcessingError {
    fn into_response(self) -> axum::response::Response {
        error!(
            "failed to download the image that requires processing. error: {}",
            self
        );

        let (status, message) = match self {
            ImageProcessingError::ClientReturnedErrorStatusCode(status, resource) => (
                StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST),
                format!("Received status code '{}' while attemtping to download the image that has to be processed: '{}'", status, resource),
            ),
            ImageProcessingError::LibvipsProcessingFailed(libvips::error::Error::InitializationError(_)) => (
                StatusCode::BAD_REQUEST,
                String::from("The image that was requested to be processed cannot be opened."),
            ),
            ImageProcessingError::ImageDownloadTimedOut => (
                StatusCode::BAD_REQUEST,
                String::from("Downloading the image requested to be processed timed out."),
            ),
            ImageProcessingError::InvalidResourceUriProvided(resource_uri) => (
                StatusCode::BAD_REQUEST,
                format!("The provided resource URI is not valid: '{}'", resource_uri)
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("Something went wrong on our side."),
            ),
        };
        let body = json!({ "error": message }).to_string();
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(body.into())
            .unwrap()
    }
}

pub async fn process_image(
    State(AppState {
        vips_app,
        image_provider,
    }): State<AppState>,
    ProcessImageRequestExtractor(params): ProcessImageRequestExtractor<ProcessImageRequest>,
) -> Result<Response<Body>, ImageProcessingError> {
    let now = SystemTime::now();
    let main_img = image_provider.get_file(&params.image_address).await?;
    let mut total_input_size = main_img.bytes.len();

    let watermarks_futures = params
        .watermarks
        .iter()
        .map(|wm| image_provider.get_file(&wm.image_address));
    let watermarks = join_all(watermarks_futures)
        .await
        .into_iter()
        .filter(|r| {
            if r.is_err() {
                warn!(
                    "failed to download watermark with error {}",
                    r.as_ref().err().unwrap()
                );
            }
            r.is_ok()
        })
        .map(|r| {
            let watermark = r.unwrap();
            total_input_size += watermark.bytes.len();
            watermark.bytes
        })
        .collect();

    if let Ok(elapsed) = now.elapsed() {
        let duration =
            (elapsed.as_secs() as f64) + f64::from(elapsed.subsec_nanos()) / 1_000_000_000_f64;
        FETCH_DURATION.success.observe(duration);
    }

    let format = params.format;

    // processing the image is a blocking operation and originally I've use the tokio::spawn_blocking option to process the image.
    // it was decently performing, but I've benchmarked rayon as well and the performance improved a lot in terms of
    // response time and memory used
    let (send, recv) = tokio::sync::oneshot::channel();
    rayon::spawn(move || {
        let image = image_processor::process_image(main_img.bytes, watermarks, params);
        let _ = send.send(image);
    });
    let processed_image = recv.await.map_err(|e| {
        error!(
            "failed to join the thread which process the image. error: {}",
            e
        );
        ImageProcessingError::ProcessingWorkerJoinError
    })?
    .map_err(|e| {
        error!(
            "the image processing has failed for the resource with the error: {}. libvips raw error is: {}",
            e, vips_app.error_buffer().unwrap_or("").replace("\n", ". ")
        );
        ImageProcessingError::LibvipsProcessingFailed(e)
    })?;

    log_size_metrics(&format, total_input_size, processed_image.len());
    let mut response_builder = Response::builder().status(StatusCode::OK);
    for (key, value) in main_img.response_headers.into_iter() {
        if !HEADERS_DETERMINED_BY_DALI.contains(&key.to_lowercase().as_str()) {
            response_builder = response_builder.header(key, value);
        }
    }
    Ok(response_builder
        .header("Content-Type", format!("image/{}", format))
        .body(Body::from(processed_image))
        .unwrap())
}

fn log_size_metrics(format: &ImageFormat, input_size: usize, response_length: usize) {
    match format {
        ImageFormat::Jpeg => {
            INPUT_SIZE.jpeg.observe(input_size as f64);
            OUTPUT_SIZE.jpeg.observe(response_length as f64);
        }
        ImageFormat::Heic => {
            INPUT_SIZE.heic.observe(input_size as f64);
            OUTPUT_SIZE.heic.observe(response_length as f64);
        }
        ImageFormat::Webp => {
            INPUT_SIZE.webp.observe(input_size as f64);
            OUTPUT_SIZE.webp.observe(response_length as f64);
        }
        ImageFormat::Png => {
            INPUT_SIZE.png.observe(input_size as f64);
            OUTPUT_SIZE.png.observe(response_length as f64);
        }
    }
}
